use crate::fmr::*;
use crate::carbon::{Carbon, Carbons};
use std::ptr;
use std::slice;
use std::ffi::CStr;
use std::os::raw::c_char;

pub type LfResult = u32;

#[repr(u32)]
enum LfStatus {
    Success = 0,
    NullPointer = 1,
    InvalidString = 2,
    PackageNotLoaded = 3,
    NoDevicesFound = 4,
    IndexOutOfBounds = 5,
}

/// Returns an opaque pointer to a set of Carbon devices and the length of the set.
///
/// There are no guarantees about the representation of the device set. The returned value should
/// be used solely as a handle to provide to other libflipper functions that accept a Carbon set.
///
/// The pointer returned as `devices` is heap-allocated and owned by the caller. The
/// proper way to release the device set is by using `carbon_release`.
pub extern "C" fn carbon_attach(devices: *mut *mut Carbons, length: *mut u32) -> LfResult {
    let carbons = Carbon::attach();
    if carbons.len() == 0 { return LfStatus::NoDevicesFound as u32; }

    let carbons = Box::new(Carbon::attach());
    let carbons_length = carbons.len();
    let carbons_pointer = Box::into_raw(carbons);

    unsafe {
        *devices = carbons_pointer;
        *length = carbons_length as u32;
    }

    LfStatus::Success as u32
}

/// Releases the memory used by the Carbon device set.
///
/// This function takes ownership of the devices pointer, then frees the backing memory. The
/// devices pointer should never be accessed after calling this function.
pub extern "C" fn carbon_release(devices: *mut Carbons) -> LfResult {
    // Re-create the Box<Carbons> from the raw pointer.
    // When this function ends the box will be dropped.
    let carbons = unsafe { Box::from_raw(devices) };
    LfStatus::Success as u32
}

/// Initializes an FMR "call packet" using the given arguments.
///
/// A call packet is a packet which, when delivered to Flipper, will cause Flipper to execute
/// a function in a module loaded on Flipper.
///
/// ### Arguments:
///
/// * `packet`: A pointer to an `FmrPacket` sized block of memory. This function mutates the
/// contents of `packet`, and so should have exclusive access to it (i.e. no other threads should
/// have access to `packet` while this function runs). This function does not take ownership of
/// the memory behind `packet`, so it is up to the caller to manage its memory.
///
/// * `module`: The index of the module loaded on Flipper which we are sending a remote call to.
/// This will need to be looked up before executing `lf_create_call`; see `lf_dyld`.
///
/// * `function`: The index of the function within the module we are preparing to call.
/// This is given by the definition of the module being used.
///
/// * `return_type`: The type of value we expect to receive by executing this call on Flipper.
///
/// * `argv`: A pointer to an array of LfArg structs describing the arguments in this call.
/// This function does not take ownership of the memory behind `argv`, so it is up to the caller
/// to manage its memory.
///
/// * `argc`: A count of the number of arguments in `argv`.
pub extern "C" fn lf_create_call(
    module: LfModule,
    function: LfFunction,
    return_type: LfType,
    argv: *const LfArg,
    argc: LfArgc,
    packet: *mut FmrPacket,
) -> LfResult {
    if argv == ptr::null() { return 1; }
    if packet == ptr::null_mut() { return 1; }

    // Convert raw C types into safe Rust types.
    let (mut packet, args) = unsafe {
        let packet = &mut *(packet as *mut FmrPacketCall);
        let args = slice::from_raw_parts(argv, argc as usize);
        (packet, args)
    };

    let status = match create_call(packet, module, function, return_type, args) {
        Ok(_) => LfStatus::Success,
        Err(_) => LfStatus::NullPointer,
    };

    status as u32
}

/// Searches for the given module on the given LfDevice and returns its index, if loaded.
///
/// ### Arguments:
///
/// `device`: A handle to an existing `LfDevice`. This function requires exclusive access to the
/// device while executing. This function does not take ownership of `device`, so it is up to the
/// caller to manage its memory.
///
/// `module`: A string that is the name of the module we want to look up on the device.
/// This function does not take ownership of `module`, so the caller is responsible for its memory.
pub extern "C" fn lf_dyld(
    device: *mut LfDevice,
    module: *const c_char,
    index: *mut u32,
) -> LfResult {
    let mut device: Box<dyn LfDevice> = unsafe { Box::from_raw(device) };

    let module_cstr = unsafe { CStr::from_ptr(module) };
    let module_string = match module_cstr.to_str() {
        Ok(module_string) => module_string,
        Err(_) => return LfStatus::InvalidString as u32,
    };

    match device.load(module_string) {
        Some(loaded_index) => unsafe { *index = loaded_index },
        None => return LfStatus::PackageNotLoaded as u32,
    }

    LfStatus::Success as u32
}
