use std::mem::size_of;
use std::ptr;
use std::mem;
use std::slice;
use std::ffi::{CStr, CString};
use std::io::{Read, Write};
use std::fmt::{self as fmt, Debug};
use std::collections::HashMap;
use std::os::raw::{c_void, c_char};

use crate::lf::Args;

const FMR_PACKET_SIZE: usize = 64;
const FMR_MAGIC_NUMBER: u8 = 0xFE;
const FMR_PAYLOAD_SIZE: usize = FMR_PACKET_SIZE - size_of::<fmr_header>();

#[derive(Copy, Clone)]
struct fmr_payload([u8; FMR_PAYLOAD_SIZE]);

impl fmr_payload {
    pub fn empty() -> fmr_payload { fmr_payload([0; FMR_PAYLOAD_SIZE]) }
}

type lf_crc_t = u16;
type lf_types = u32;
type lf_return = u64;
type lf_argc = u8;
type lf_arg_repr = u64;
type lf_module = u32;
type lf_function = u8;

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
enum fmr_class {
    rpc = 0,
    push = 1,
    pull = 2,
    dyld = 3,
    malloc = 4,
    free = 5,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct fmr_header {
    magic: u8,
    crc: lf_crc_t,
    len: u16,
    kind: fmr_class,
}

union fmr_packet {
    base: fmr_packet_base,
    call: fmr_packet_call,
    data: fmr_packet_push_pull,
    dyld: fmr_packet_dyld,
    memory: fmr_packet_memory,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct fmr_packet_base {
    header: fmr_header,
    payload: fmr_payload,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct fmr_call {
    module: u8,
    function: u8,
    ret: lf_type,
    argt: lf_types,
    argc: lf_argc,
    argv: (),
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct fmr_packet_call {
    header: fmr_header,
    call: fmr_call,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct fmr_packet_push_pull {
    header: fmr_header,
    len: u32,
    ptr: u64,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct fmr_packet_dyld {
    header: fmr_header,
    module: *mut c_char,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct fmr_packet_memory {
    header: fmr_header,
    size: u32,
    ptr: u64,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct fmr_result {
    value: lf_return,
    error: u8,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct lf_arg {
    pub(crate) kind: lf_type,
    pub(crate) value: lf_arg_repr,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum lf_type {
    void = 2,
    int = 4,
    ptr = 6,

    // Unsigned types
    uint8 = 0,
    uint16 = 1,
    uint32 = 3,
    uint64 = 7,

    // Signed types
    int8 = 8,
    int16 = 9,
    int32 = 11,
    int64 = 15,
}

impl lf_type {
    const MAX: u8 = 15;

    fn size(&self) -> usize {
        match self {
            lf_type::int8 | lf_type::uint8 => 1,
            lf_type::int16 | lf_type::uint16 => 2,
            lf_type::int32 | lf_type::uint32 => 4,
            lf_type::int64 |
            lf_type::uint64 |
            lf_type::ptr |
            lf_type::void => 8,
            _ => 0,
        }
    }
}

impl Debug for fmr_payload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for chunk in self.0.chunks(8) {
            for byte in chunk { write!(f, "{:02X} ", byte)?; }
            writeln!(f);
        }
        Ok(())
    }
}

pub struct Module {
    name: String,
    index: u32,
    version: u16,
}

impl Module {
    pub fn new(name: String, index: u32, version: u16) -> Module {
        Module { name, index, version }
    }
}

pub struct Modules(HashMap<String, Module>);

impl Modules {
    pub fn new() -> Modules { Modules(HashMap::new()) }

    pub fn register(&mut self, module: Module) {
        self.0.insert(module.name.clone(), module);
    }

    pub fn find(&self, name: &str) -> Option<u32> {
        self.0.get(name).map(|module| module.index)
    }

    pub fn unload(&mut self, name: &str) -> bool {
        self.0.remove(name).is_some()
    }
}

pub extern "C" fn lf_crc(data: *const c_void, length: u32) -> u16 {
    const POLY: u16 = 0x1021;
    let mut crc: u16 = 0;
    for i in 0..length {
        unsafe {
            let word = (data.offset(i as isize) as *const u16).read();
            crc ^= word << 8;
            for _ in 0..=8 {
                if crc & 0x8000 != 0 {
                    crc = crc << 1 ^ POLY;
                } else {
                    crc = crc << 1;
                }
            }
        }
    }
    crc
}

pub extern "C" fn lf_create_call(
    module: lf_module,
    function: lf_function,
    ret: lf_type,
    argv: *const lf_arg,
    argc: lf_argc,
    header: *mut fmr_header,
    call: *mut fmr_call
) -> i32 {
    let mut offset = unsafe {
        (*call).module = module as u8;
        (*call).function = function;
        (*call).ret = ret;
        (*call).argc = argc;
        &mut ((*call).argv) as *mut () as *mut u8
    };

    for i in 0..argc {
        unsafe {
            let arg = argv.offset(i as isize);
            (*call).argt |= ((((*arg).kind as u8) & lf_type::MAX) as u32) << (i * 4);
            let size = (*arg).kind.size();
            let value_addr = &((*arg).value) as *const u64;
            ptr::copy(value_addr as *const u8, offset, size);
            offset = offset.add(size);
            (*header).len += size as u16;
        }
    }

    1
}

#[no_mangle]
pub extern "C" fn lf_dyld<T: LfDevice>(
    device: *mut T,
    module: *const c_char,
    index: *mut u32,
) -> i32 {

    let mut packet = fmr_packet {
        base: fmr_packet_base {
            header: fmr_header {
                magic: FMR_MAGIC_NUMBER,
                len: size_of::<fmr_header>() as u16,
                crc: 0,
                kind: fmr_class::dyld,
            },
            payload: fmr_payload::empty()
        }
    };

    let mut result: fmr_result = unsafe { mem::uninitialized() };

    unsafe {
        let dyld_packet = &mut packet as *mut fmr_packet as *mut fmr_packet_dyld;

        let cmodule: &CStr = CStr::from_ptr(module);
//        let cstr: &str = cmodule.to_str().expect("should get cstring");
//        let len = str::len(cstr) + 1;
        let len = 4;

//        ptr::copy(module, (*dyld_packet).module as *mut i8, len);
        (*dyld_packet).header.len += len as u16;

        let len = (*dyld_packet).header.len;
        let crc = lf_crc(&packet as *const fmr_packet as *const c_void, len as u32);
        (*dyld_packet).header.crc = crc;

        let packet_slice = slice::from_raw_parts(&packet as *const fmr_packet as *const u8, len as usize);
        Write::write(&mut *device, packet_slice);

        let mut result_buffer = [0u8; size_of::<fmr_result>()];
        Read::read(&mut *device, &mut result_buffer);
        ptr::copy(&result_buffer as *const u8, &mut result as *mut fmr_result as *mut u8, result_buffer.len());

        *index = result.value as u32;
    }

    0
}

pub trait LfDevice: Read + Write + Sized {
    fn modules(&mut self) -> &mut Modules;

    fn invoke(
        &mut self,
        module: &str,
        function: lf_function,
        ret: lf_type,
        args: Args,
    ) -> i32 {

        let mut packet = fmr_packet {
            base: fmr_packet_base {
                header: fmr_header {
                    magic: FMR_MAGIC_NUMBER,
                    len: size_of::<fmr_header>() as u16,
                    crc: 0,
                    kind: fmr_class::rpc
                },
                payload: fmr_payload([0; FMR_PAYLOAD_SIZE]),
            }
        };

        let argv: Vec<_> = args.iter().map(|arg| arg.0).collect();

        let (header, call) = unsafe {
            let header = &mut packet.call.header as *mut fmr_header;
            let call = &mut packet.call.call as *mut fmr_call;
            (header, call)
        };

        let module = self.load(module).expect("should get module");

        lf_create_call(module, function, ret, argv.as_ptr(), argv.len() as u8, header, call);
        println!("Packet header: {:?}", unsafe { packet.base.header });
        println!("Packet payload: {:?}", unsafe { packet.base.payload });

        let packet_slice: &[u8] = unsafe {
            let data = &packet as *const fmr_packet as *const u8;
            let len = packet.base.header.len as usize;
            slice::from_raw_parts(data, len)
        };

        self.write(packet_slice);

        0
    }

    /// Given a module name, returns the index of that module on this device if the module is
    /// installed. Otherwise, returns none.
    fn load(&mut self, module: &str) -> Option<u32> {
        let modules = self.modules();
        if let Some(module) = modules.find(module) { return Some(module); }

        let mut index: u32 = 0;
        let module = CString::new(module).expect("should be a valid string");

        let result = lf_dyld(self as *mut _, module.as_ptr(), &mut index as *mut u32);

        if result == 0 {
            Some(index)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_call() {
        let args = vec![
            lf_arg { kind: lf_type::uint8, value: 10 },
            lf_arg { kind: lf_type::uint16, value: 1000 },
            lf_arg { kind: lf_type::uint32, value: 2000 },
            lf_arg { kind: lf_type::uint64, value: 4000 },
        ];

        let mut packet = fmr_packet {
            base: fmr_packet_base {
                header: fmr_header {
                    magic: FMR_MAGIC_NUMBER,
                    crc: 0,
                    len: 0,
                    kind: fmr_class::rpc,
                },
                payload: fmr_payload([0; FMR_PAYLOAD_SIZE]),
            }
        };

        let mut fmr_packet_call = &mut packet as *mut fmr_packet as *mut fmr_packet_call;
        let (header, call) = unsafe {
            let header = &mut (*fmr_packet_call).header as *mut fmr_header;
            let call = &mut (*fmr_packet_call).call as *mut fmr_call;
            (header, call)
        };

        lf_create_call(3, 5, lf_type::void, args.as_ptr(), args.len() as u8, header, call);

        let payload = unsafe { packet.base.payload };
        for chunk in payload.chunks(8) {
            for byte in chunk {
                print!("{:02X} ", byte);
            }
            println!();
        }
    }
}
