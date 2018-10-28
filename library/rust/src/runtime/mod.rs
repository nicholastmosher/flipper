pub mod protocol;

use self::protocol::*;

use std::ptr;
use std::ops::Deref;
use std::ffi::CString;
use std::io::{Read, Write};
use std::collections::HashMap;
use std::os::raw::c_char;

pub trait LfDevice: Read + Write {
    fn modules(&mut self) -> &mut Modules;

    fn invoke(
        &mut self,
        module: &str,
        function: LfFunction,
        ret: LfType,
        args: Args,
    ) -> Option<u32> {

        // Create a call packet
        let packet = FmrPacket::new(FmrClass::call);
        let mut call_packet = FmrPacketCall::from(packet);

        // Write the module index and function arguments into the packet
        let module = self.load(module).expect("should get module");
        let argv: Vec<_> = args.iter().map(|arg| arg.0).collect();
        create_call(&mut call_packet, module as u32, function, ret, &argv);

        // Calculate the crc for the packet
        let len = call_packet.header.len as u32;
        let crc = calculate_crc(&call_packet as *const _ as *const u8, len);
        call_packet.header.crc = crc;

        // Send the packet as raw bytes
        self.write(unsafe { call_packet.as_bytes() });

        // Receive the result as raw bytes
        let mut result = FmrReturn::new();
        self.read(unsafe { result.as_mut_bytes() });

        Some(result.value as u32)
    }

    /// Given a module name, returns the index of that module on this device if the module is
    /// installed. Otherwise, returns none.
    fn load(&mut self, module: &str) -> Option<u32> {
        let modules = self.modules();
        if let Some(module) = modules.find(module) { return Some(module); }

        // Create a dyld packet
        let packet = FmrPacket::new(FmrClass::dyld);
        let mut dyld_packet = FmrPacketDyld::from(packet);

        let module_cstring = match CString::new(module) {
            Ok(cstr) => cstr,
            Err(_) => return None,
        };

        // Copy the module name into the packet
        let buffer = module_cstring.as_bytes_with_nul();
        let module_cstr = unsafe { &mut (dyld_packet.module) as *mut *mut c_char as *mut u8 };
        unsafe { ptr::copy(buffer.as_ptr(), module_cstr, buffer.len()) };
        dyld_packet.header.len += buffer.len() as u16;

        // Calculate the crc for the packet
        let len = dyld_packet.header.len as u32;
        let crc = calculate_crc(&dyld_packet as *const _ as *const u8, len);
        dyld_packet.header.crc = crc;

        // Send the packet as raw bytes
        self.write(unsafe { dyld_packet.as_bytes() });

        // Receive the result as raw bytes
        let mut result = FmrReturn::new();
        self.read(unsafe { result.as_mut_bytes() });

        if result.error != 0 { return None; }

        // Register this module so we don't have to look it up in the future
        let modules = self.modules();
        let module_index = result.value as u32;
        let module = Module::new(module.to_string(), module_index, 0);
        modules.register(module);

        Some(result.value as u32)
    }

    /// Pushes a buffer of data to a location in Flipper's memory space.
    ///
    /// The given pointer must be a valid location in Flipper's memory, obtained by using
    /// `LfDevice::malloc`.
    ///
    /// The data buffer to write must be no larger than the size of the memory allocated from
    /// Flipper. If the pointer being used was obtained using `device.malloc(size)`, then
    /// `data.len()` must be less than or equal to `size`.
    fn push(&mut self, pointer: LfPointer, data: &[u8]) -> Option<()> {

        // Create a push packet
        let packet = FmrPacket::new(FmrClass::push);
        let mut push_packet = FmrPacketPushPull::from(packet);

        // Write the length and address of the target memory buffer into the packet
        push_packet.len = data.len() as u32;
        push_packet.ptr = pointer.0 as u64;

        // Calculate the crc for the packet
        let len = push_packet.header.len as u32;
        let crc = calculate_crc(&push_packet as *const _ as *const u8, len);
        push_packet.header.crc = crc;

        // Write the packet as raw bytes
        self.write(unsafe { push_packet.as_bytes() });

        // Write the push payload as raw bytes
        self.write(data);

        // Read the result as raw bytes
        let mut result = FmrReturn::new();
        self.read(unsafe { result.as_mut_bytes() });

        if result.error != 0 { return None; }
        Some(())
    }

    /// Pulls a buffer of data from a location in Flipper's memory space.
    ///
    /// The given pointer must be a valid location in Flipper's memory, obtained by using
    /// `LfDevice::malloc`.
    ///
    /// The local buffer to write to must be no larger than the size of the memory allocated from
    /// Flipper. If the pointer being used was obtained using `device.malloc(size)`, then
    /// `data.len()` must be less than or equal to `size`.
    fn pull(&mut self, pointer: LfPointer, buffer: &mut [u8]) -> Option<()> {

        // Create a pull packet
        let packet = FmrPacket::new(FmrClass::pull);
        let mut pull_packet = FmrPacketPushPull::from(packet);

        // Write the length and address of the target memory buffer into the packet
        pull_packet.len = buffer.len() as u32;
        pull_packet.ptr = pointer.0 as u64;

        // Calculate the crc for the packet
        let len = pull_packet.header.len as u32;
        let crc = calculate_crc(&pull_packet as *const _ as *const u8, len);
        pull_packet.header.crc = crc;

        // Write the packet as raw bytes
        self.write(unsafe { pull_packet.as_bytes() });

        // Read the pull payload as raw bytes
        self.read(buffer);

        // Read the result as raw bytes
        let mut result = FmrReturn::new();
        self.read(unsafe { result.as_mut_bytes() });

        if result.error != 0 { return None; }
        Some(())
    }

    /// Allocates a buffer of data of the given size in Flipper's memory space.
    fn malloc(&mut self, size: u32) -> Option<LfPointer> {

        // Create a malloc packet
        let packet = FmrPacket::new(FmrClass::malloc);
        let mut malloc_packet = FmrPacketMemory::from(packet);

        // Write the size of the requested buffer in the packet
        malloc_packet.size = size;

        // Calculate the crc for the packet
        let len = malloc_packet.header.len as u32;
        let crc = calculate_crc(&malloc_packet as *const _ as *const u8, len);
        malloc_packet.header.crc = crc;

        // Send the packet as raw bytes
        self.write(unsafe { malloc_packet.as_bytes() });

        // Read the result as raw bytes
        let mut result = FmrReturn::new();
        self.read(unsafe { result.as_mut_bytes() });

        if result.error != 0 { return None; }
        Some(LfPointer(result.value as u32))
    }

    /// Frees a buffer of memory in Flipper's memory space.
    fn free(&mut self, pointer: LfPointer) -> Option<()> {

        // Create a free packet
        let packet = FmrPacket::new(FmrClass::free);
        let mut free_packet = FmrPacketMemory::from(packet);

        // Write the address of the buffer to free into the packet
        free_packet.ptr = pointer.0 as u64;

        // Calculate the crc for the packet
        let len = free_packet.header.len as u32;
        let crc = calculate_crc(&free_packet as *const _ as *const u8, len);
        free_packet.header.crc = crc;

        // Send the packet as raw bytes
        self.write(unsafe { free_packet.as_bytes() });

        // Read the result as raw bytes
        let mut result = FmrReturn::new();
        self.read(unsafe { result.as_mut_bytes() });

        if result.error != 0 { return None; }
        Some(())
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

/// Given a memory buffer and a length, generates a CRC of the data in the buffer.
pub fn calculate_crc(data: *const u8, length: u32) -> u16 {
    const POLY: u16 = 0x1021;
    let mut crc: u16 = 0;
    for i in 0..length {
        unsafe {
            let word = ptr::read(data.offset(i as isize) as *const u16);
            crc = crc ^ word << 8;
            for _ in 0..8 {
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
