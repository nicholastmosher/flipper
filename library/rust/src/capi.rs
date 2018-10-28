use std::ptr;
use std::slice;
use std::ffi::CStr;
use std::os::raw::{c_void, c_char};

use crate::runtime::{LfDevice, create_call};
use crate::runtime::protocol::*;
use crate::carbon::{Carbon, Carbons};
use crate::device::{UsbDevice, AtsamDevice};

pub type LfResult = u32;

#[repr(u32)]
pub enum LfStatus {
    Success = 0,
    NullPointer = 1,
    InvalidString = 2,
    PackageNotLoaded = 3,
    NoDevicesFound = 4,
    IndexOutOfBounds = 5,
    IllegalType = 6,
}

/// Returns an opaque pointer to a list of Carbon devices and the length of the list.
///
/// There are no guarantees about the representation of the device list. The returned value should
/// be used solely as a handle to provide to other libflipper functions that accept a Carbon list.
///
/// The pointer returned as `devices` is heap-allocated and owned by the caller. The
/// proper way to release the device list is by using `carbon_release`.
#[no_mangle]
pub extern "C" fn carbon_attach(devices: *mut *mut c_void, length: *mut u32) -> LfResult {
    let carbons = Carbon::attach();
    if carbons.len() == 0 { return LfStatus::NoDevicesFound as u32; }

    let carbons = Box::new(Carbon::attach());
    let carbons_length = carbons.len();
    let carbons_pointer = Box::into_raw(carbons);

    unsafe {
        *devices = carbons_pointer as *mut c_void;
        *length = carbons_length as u32;
    }

    LfStatus::Success as u32
}

/// Retrieves a device from the device list at the given index. Index 0 is the first device.
///
/// The returned handle represents a single attached Carbon device. This handle is only valid
/// while the device list it came from is valid. That is, if `carbon_release(devices)` is called,
/// then the Carbon handle that was returned by this function is no longer valid.
///
/// If the given devices pointer is NULL, then NULL is returned.
///
/// If the given index is out of bounds for the device list, then NULL is returned.
#[no_mangle]
pub extern "C" fn carbon_select(devices: *mut c_void, index: u32) -> *mut c_void {
    if devices == ptr::null_mut() { return ptr::null_mut() }

    let devices = unsafe { &mut *(devices as *mut Carbons) };
    match devices.get_mut(index as usize) {
        Some(device) => device as *mut _ as *mut c_void,
        None => ptr::null_mut(),
    }
}

/// Given a handle to a Carbon device, returns a handle to the inner atmegau2 LfDevice.
///
/// This function does not take ownership of the Carbon device passed to it, so the caller is
/// responsible for managing its memory.
///
/// The returned handle to the atmegau2 device is heap-allocated and must be freed with
/// `carbon_release`.
///
/// The returned handle to the atmegau2 device is valid only as long as its parent Carbon device
/// is valid, which is in turn only valid for as long as the containing Carbon devices list is.
/// In other words, for a device list `devices`, once `carbon_release(devices)` has been called,
/// the LfDevice handle that was returned from this function will be invalid.
#[no_mangle]
pub extern "C" fn carbon_atmegau2(carbon: *mut c_void) -> *mut c_void {
    if carbon == ptr::null_mut() { return ptr::null_mut() }
    let carbon = unsafe { &mut *(carbon as *mut Carbon) };
    let atmegau2 = carbon.atmegau2() as *mut LfDevice;
    let boxed_atmegau2 = Box::new(atmegau2);
    Box::into_raw(boxed_atmegau2) as *mut c_void
}

/// Given a handle to a Carbon device, returns a handle to the inner atsam4s LfDevice.
///
/// This function does not take ownership of the Carbon device passed to it, so the caller is
/// responsible for managing its memory.
///
/// The returned handle to the atsam4s device is heap-allocated and must be freed with
/// `carbon_release`.
///
/// The returned handle to the atsam4s device is valid only as long as its parent Carbon device
/// is valid, which is in turn only valid for as long as the containing Carbon devices list is.
/// In other words, for a device list `devices`, once `carbon_release(devices)` has been called,
/// the LfDevice handle that was returned from this function will be invalid.
#[no_mangle]
pub extern "C" fn carbon_atsam4s(carbon: *mut c_void) -> *mut c_void {
    if carbon == ptr::null_mut() { return ptr::null_mut() }
    let carbon = unsafe { &mut *(carbon as *mut Carbon) };
    let atsam4s = carbon.atsam4s() as *mut LfDevice;
    let boxed_atsam4s = Box::new(atsam4s);
    Box::into_raw(boxed_atsam4s) as *mut c_void
}

/// Releases the memory used by the boxed resource.
///
/// This function takes ownership of the resource pointer, then frees the backing memory. The
/// pointer should never be accessed after calling this function.
#[no_mangle]
pub extern "C" fn carbon_release(resource: *mut c_void) -> LfResult {
    // Re-create the Box<_> from the raw pointer.
    // When this function ends the box will be dropped.
    let _ = unsafe { Box::from_raw(resource as *mut _) };
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
#[no_mangle]
pub extern "C" fn lf_create_call(
    module: u32,
    function: u8,
    return_type: u8,
    argv: *const c_void,
    argc: u8,
    packet: *mut c_void,
) -> LfResult {
    if argv == ptr::null() { return 1; }
    if packet == ptr::null_mut() { return 1; }

    // Convert raw C types into safe Rust types.
    let (packet, args) = unsafe {
        let packet = &mut *(packet as *mut FmrPacketCall);
        let args = slice::from_raw_parts(argv as *const LfArg, argc as usize);
        (packet, args)
    };

    let return_type = match LfType::from(return_type) {
        Some(ret) => ret,
        None => return LfStatus::IllegalType as LfResult,
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
#[no_mangle]
pub extern "C" fn lf_dyld(
    device: *mut c_void,
    module: *const c_char,
    index: *mut u32,
) -> LfResult {
    let mut device: Box<Box<dyn LfDevice>> = unsafe { Box::from_raw(device as *mut _) };

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

/// Given a memory buffer and a length, generates a CRC of the data in the buffer.
#[no_mangle]
pub extern "C" fn lf_crc(data: *const u8, length: u32) -> u16 {
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

