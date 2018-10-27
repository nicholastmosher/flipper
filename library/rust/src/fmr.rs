use std::ptr;
use std::slice;
use std::mem::size_of;
use std::ffi::CString;
use std::io::{Read, Write};
use std::fmt::{self as fmt, Debug};
use std::collections::HashMap;
use std::os::raw::c_char;

use crate::capi::lf_crc;
use crate::lf::Args;

const FMR_PACKET_SIZE: usize = 64;
const FMR_MAGIC_NUMBER: u8 = 0xFE;
const FMR_PAYLOAD_SIZE: usize = FMR_PACKET_SIZE - size_of::<FmrHeader>();

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct FmrPayload([u8; FMR_PAYLOAD_SIZE]);

const FMR_PAYLOAD_EMPTY: FmrPayload = FmrPayload([0; FMR_PAYLOAD_SIZE]);

pub type LfCrc = u16;
pub type LfTypes = u32;
pub type LfValue = u64;
pub type LfArgc = u8;
pub type LfArgRepr = u64;
pub type LfModule = u32;
pub type LfFunction = u8;

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum FmrClass {
    call = 0,
    push = 1,
    pull = 2,
    dyld = 3,
    malloc = 4,
    free = 5,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrHeader {
    pub magic: u8,
    pub crc: LfCrc,
    pub len: u16,
    pub kind: FmrClass,
}

pub union FmrPacket {
    base: FmrPacketBase,
    call: FmrPacketCall,
    data: FmrPacketPushPull,
    dyld: FmrPacketDyld,
    memory: FmrPacketMemory,
}

impl FmrPacket {
    pub fn new(class: FmrClass) -> FmrPacket {
        FmrPacket {
            base: FmrPacketBase {
                header: FmrHeader {
                    magic: FMR_MAGIC_NUMBER,
                    crc: 0,
                    // Under normal circumstances this would be mem::size_of::<FmrHeader>(),
                    // but for some reason the packed repr in C calculates the size as 8, not 6.
                    len: 8,
                    kind: class,
                },
                payload: FMR_PAYLOAD_EMPTY,
            }
        }
    }

    pub unsafe fn into_call(mut self) -> FmrPacketCall {
        *(&mut self as *mut FmrPacket as *mut FmrPacketCall)
    }

    pub unsafe fn into_push_pull(mut self) -> FmrPacketPushPull {
        *(&mut self as *mut FmrPacket as *mut FmrPacketPushPull)
    }

    pub unsafe fn into_dyld(mut self) -> FmrPacketDyld {
        *(&mut self as *mut FmrPacket as *mut FmrPacketDyld)
    }

    pub unsafe fn into_memory(mut self) -> FmrPacketMemory {
        *(&mut self as *mut FmrPacket as *mut FmrPacketMemory)
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrPacketBase {
    pub header: FmrHeader,
    pub payload: FmrPayload,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrCall {
    pub module: u8,
    pub function: u8,
    pub ret: LfType,
    pub argt: LfTypes,
    pub argc: LfArgc,
    pub argv: (),
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrPacketCall {
    pub header: FmrHeader,
    pub call: FmrCall,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrPacketPushPull {
    pub header: FmrHeader,
    pub len: u32,
    pub ptr: u64,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrPacketDyld {
    pub header: FmrHeader,
    pub module: *mut c_char,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrPacketMemory {
    pub header: FmrHeader,
    pub size: u32,
    pub ptr: u64,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrReturn {
    pub value: LfValue,
    pub error: u8,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct LfArg {
    pub kind: LfType,
    pub value: LfArgRepr,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum LfType {
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

impl LfType {
    pub const MAX: u8 = 15;

    pub fn size(&self) -> usize {
        match self {
            LfType::int8 | LfType::uint8 => 1,
            LfType::int16 | LfType::uint16 => 2,
            LfType::int32 | LfType::uint32 => 4,
            LfType::int64 |
            LfType::uint64 |
            LfType::ptr |
            LfType::void => 8,
            _ => 0,
        }
    }
}

impl Debug for FmrPayload {
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

pub trait LfDevice: Read + Write {
    fn modules(&mut self) -> &mut Modules;

    fn invoke(
        &mut self,
        module: &str,
        function: LfFunction,
        ret: LfType,
        args: Args,
    ) -> Option<u32> {
        let packet = FmrPacket::new(FmrClass::call);
        let mut call_packet = unsafe { packet.into_call() };

        let argv: Vec<_> = args.iter().map(|arg| arg.0).collect();
        let module = self.load(module).expect("should get module");

        create_call(&mut call_packet, module as u32, function, ret, &argv);

        // Calculate the crc for the packet
        let len = call_packet.header.len as u32;
        let crc = lf_crc(&call_packet as *const FmrPacketCall as *const u8, len);
        call_packet.header.crc = crc;

        // Send the packet as raw bytes
        let packet_slice: &[u8] = unsafe {
            let data = &call_packet as *const FmrPacketCall as *const u8;
            slice::from_raw_parts(data, size_of::<FmrPacket>())
        };
        self.write(packet_slice);

        // Receive the result as raw bytes
        let result = unsafe {
            let mut result = FmrReturn { value: 0, error: 0 };
            let result_pointer = &mut result as *mut FmrReturn as *mut u8;
            let result_slice = slice::from_raw_parts_mut(result_pointer, size_of::<FmrReturn>());
            self.read(result_slice);
            result
        };

        Some(result.value as u32)
    }

    /// Given a module name, returns the index of that module on this device if the module is
    /// installed. Otherwise, returns none.
    fn load(&mut self, module: &str) -> Option<u32> {
        let modules = self.modules();
        if let Some(module) = modules.find(module) { return Some(module); }

        // Convert one union variant to another
        let packet = FmrPacket::new(FmrClass::dyld);
        let mut dyld_packet = unsafe { packet.into_dyld() };

        let module_cstring = match CString::new(module) {
            Ok(cstr) => cstr,
            Err(_) => return None,
        };

        // Copy the module name into the packet
        let buffer = module_cstring.as_bytes_with_nul();
        let module = unsafe { &mut (dyld_packet.module) as *mut *mut c_char as *mut u8 };
        unsafe { ptr::copy(buffer.as_ptr(), module, buffer.len()) };
        dyld_packet.header.len += buffer.len() as u16;

        // Calculate the crc for the packet
        let len = dyld_packet.header.len as u32;
        let crc = lf_crc(&dyld_packet as *const FmrPacketDyld as *const u8, len);
        dyld_packet.header.crc = crc;

        // Send the packet as raw bytes
        let packet_buffer: &[u8] = unsafe {
            let data = &dyld_packet as *const FmrPacketDyld as *const u8;
            slice::from_raw_parts(data, size_of::<FmrPacket>())
        };
        self.write(packet_buffer);

        // Receive the result as raw bytes
        let result = unsafe {
            let mut result = FmrReturn { value: 0, error: 0 };
            let result_pointer = &mut result as *mut FmrReturn as *mut u8;
            let result_slice = slice::from_raw_parts_mut(result_pointer, size_of::<FmrReturn>());
            self.read(result_slice);
            result
        };

        if result.error != 0 { return None }
        Some(result.value as u32)
    }
}

pub fn create_call(
    packet: &mut FmrPacketCall,
    module: LfModule,
    function: LfFunction,
    return_type: LfType,
    args: &[LfArg],
) -> Result<(), ()> {
    let argc = args.len() as LfArgc;

    // Populate call packet
    packet.call.module = module as u8;
    packet.call.function = function;
    packet.call.ret = return_type;
    packet.call.argc = argc;

    // Take the offset to the base of the argument list
    let mut offset = &mut packet.call.argv as *mut () as *mut u8;

    // Copy each argument into the call packet
    for i in 0..argc {
        let arg: &LfArg = args.get(i as usize).ok_or(())?;
        packet.call.argt |= (((arg.kind as u8) & LfType::MAX) as u32) << (i * 4);

        // Copy the argument value into the call packet
        let arg_size = arg.kind.size();
        unsafe {
            let arg_value_address = &arg.value as *const u64;
            ptr::copy(arg_value_address as *const u8, offset, arg_size);

            // Increase the offset and size of the packet by the size of this argument
            offset = offset.add(arg_size);
            packet.header.len += arg_size as u16;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_call() {
        let args = vec![
            LfArg { kind: LfType::uint8, value: 10 },
            LfArg { kind: LfType::uint16, value: 1000 },
            LfArg { kind: LfType::uint32, value: 2000 },
            LfArg { kind: LfType::uint64, value: 4000 },
        ];

        let mut packet = FmrPacket::new(FmrClass::call);
        let mut call_packet = unsafe { packet.into_call() };
        create_call(&mut call_packet, 3, 5, LfType::void, &args);

        let payload = unsafe { packet.base.payload };
        for chunk in payload.chunks(8) {
            for byte in chunk {
                print!("{:02X} ", byte);
            }
            println!();
        }
    }
}
