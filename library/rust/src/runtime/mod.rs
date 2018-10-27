pub mod protocol;
use self::protocol::*;

use std::ptr;
use std::slice;
use std::ops::Deref;
use std::mem::size_of;
use std::ffi::CString;
use std::io::{Read, Write};
use std::collections::HashMap;
use std::os::raw::c_char;

use crate::capi::lf_crc;

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

/// Represents an argument to a remote call.
///
/// Any type which implement `Into<Arg>` can be appended to an `Args` list.
/// This currently includes `u8`, `u16`, `u32`, and `u64`.
///
/// ```
/// use flipper::lf::Arg;
///
/// let one =   Arg::from(10 as u8);
/// let two =   Arg::from(20 as u16);
/// let three = Arg::from(30 as u32);
/// let four =  Arg::from(40 as u64);
/// ```
pub struct Arg(pub(crate) LfArg);

impl From<u8> for Arg {
    fn from(value: u8) -> Arg {
        Arg(LfArg {
            kind: LfType::uint8,
            value: value as LfValue,
        })
    }
}

impl From<u16> for Arg {
    fn from(value: u16) -> Arg {
        Arg(LfArg {
            kind: LfType::uint16,
            value: value as LfValue,
        })
    }
}

impl From<u32> for Arg {
    fn from(value: u32) -> Arg {
        Arg(LfArg {
            kind: LfType::uint32,
            value: value as LfValue,
        })
    }
}

impl From<u64> for Arg {
    fn from(value: u64) -> Arg {
        Arg(LfArg {
            kind: LfType::uint64,
            value: value as LfValue,
        })
    }
}

impl From<LfPointer> for Arg {
    fn from(address: LfPointer) -> Self {
        Arg(LfArg {
            kind: LfType::ptr,
            value: address.0 as LfValue,
        })
    }
}

/// Represents an ordered, typed set of arguments to a Flipper remote call. This is
/// to be used for calling `invoke`.
///
/// ```
/// let args = Args::new()
///                .append(10 as u8)
///                .append(20 as u16)
///                .append(30 as u32)
///                .append(40 as u64);
/// ```
pub struct Args(Vec<Arg>);

impl Args {
    pub fn new() -> Self {
        Args(Vec::new())
    }
    pub fn append<T: Into<Arg>>(mut self, arg: T) -> Self {
        self.0.push(arg.into());
        self
    }
}

impl Deref for Args {
    type Target = Vec<Arg>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A container type for a value returned by performing an
/// `invoke` call. Types which implement `LfReturnable`
/// must define how to extract their own representation
/// from this container.
pub struct LfReturn(pub(crate) LfValue);

/// A trait to be implemented for types which can be returned
/// from an `invoke` call. Currently, only types up to
/// 64 bits can be represented. Any type which implements
/// `LfReturnable` must be able to extract itself from the
/// 64 bit representation in `LfReturn`.
///
/// Current `LfReturnable` types are `()`, `u8,` `u16`, `u32`,
/// and `u64`.
pub trait LfReturnable: From<LfReturn> {
    fn lf_type() -> LfType;
}

impl LfReturnable for () {
    fn lf_type() -> LfType { LfType::void }
}

impl From<LfReturn> for () {
    fn from(_: LfReturn) -> Self { () }
}

impl LfReturnable for u8 {
    fn lf_type() -> LfType { LfType::uint8 }
}

impl From<LfReturn> for u8 {
    fn from(ret: LfReturn) -> Self {
        ret.0 as u8
    }
}

impl LfReturnable for u16 {
    fn lf_type() -> LfType { LfType::uint16 }
}

impl From<LfReturn> for u16 {
    fn from(ret: LfReturn) -> Self {
        ret.0 as u16
    }
}

impl LfReturnable for u32 {
    fn lf_type() -> LfType { LfType::uint32 }
}

impl From<LfReturn> for u32 {
    fn from(ret: LfReturn) -> Self {
        ret.0 as u32
    }
}

impl LfReturnable for u64 {
    fn lf_type() -> LfType { LfType::uint64 }
}

impl From<LfReturn> for u64 {
    fn from(ret: LfReturn) -> Self { ret.0 as u64 }
}

impl LfReturnable for LfPointer {
    fn lf_type() -> LfType { LfType::ptr }
}

impl From<LfReturn> for LfPointer {
    fn from(ret: LfReturn) -> Self {
        LfPointer(ret.0 as LfAddress)
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
