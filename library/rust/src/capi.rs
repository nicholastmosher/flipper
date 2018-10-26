use crate::fmr::*;
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
}

pub extern "C" fn lf_create_call(
    module: LfModule,
    function: LfFunction,
    ret: LfType,
    argv: *const LfArg,
    argc: LfArgc,
    packet: *mut FmrPacket
) -> LfResult {
    if argv == ptr::null() { return 1 }
    if packet == ptr::null_mut() { return 1 }

    // Convert raw C types into safe Rust types.
    let (packet, args) = unsafe {
        let packet = &mut *packet;
        let args = slice::from_raw_parts(argv, argc as usize);
        (packet, args)
    };

    let status = match create_call(packet, module, function, ret, args) {
        Ok(_) => LfStatus::Success,
        Err(_) => LfStatus::NullPointer,
    };

    status as u32
}

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
