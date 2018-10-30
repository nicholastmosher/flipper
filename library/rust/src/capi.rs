use std::mem;
use std::ptr;
use std::slice;
use std::ffi::CStr;
use std::os::raw::{c_void, c_char};

use crate::runtime::{LfDevice, Args, create_call};
use crate::runtime::protocol::*;
use crate::carbon::{Carbon, Carbons};
use crate::device::{UsbDevice, AtsamDevice};

#[repr(u32)]
pub enum LfResult {
    Success = 0,
    NullPointer = 1,
    InvalidString = 2,
    PackageNotLoaded = 3,
    NoDevicesFound = 4,
    IndexOutOfBounds = 5,
    IllegalType = 6,
    InvocationError = 7,
}

/// Returns an opaque pointer to a list of Carbon devices and the length of the list.
///
/// There are no guarantees about the representation of the device list. The returned value should
/// be used solely as a handle to provide to other functions that accept a Carbon list.
///
/// The pointer returned as `devices` is heap-allocated and owned by the caller. The
/// proper way to release the device list is by using `lf_release`.
#[no_mangle]
pub extern "C" fn lf_attach_carbons_usb(devices: *mut *mut c_void, length: *mut u32) -> LfResult {
    let carbons = Carbon::attach();
    if carbons.len() == 0 { return LfResult::NoDevicesFound; }

    let carbons = Box::new(Carbon::attach());
    let carbons_length = carbons.len();
    let carbons_pointer = Box::into_raw(carbons);

    unsafe {
        *devices = carbons_pointer as *mut c_void;
        *length = carbons_length as u32;
    }

    LfResult::Success
}

/// Retrieves a device from the device list at the given index. Index 0 is the first device.
///
/// The returned handle represents a single attached Carbon device. This handle is only valid
/// while the device list it came from is valid. That is, if `lf_release(devices)` is called,
/// then the Carbon handle that was returned by this function is no longer valid.
///
/// If the given devices pointer is NULL, then NULL is returned.
///
/// If the given index is out of bounds for the device list, then NULL is returned.
#[no_mangle]
pub extern "C" fn lf_select_carbon(devices: *mut c_void, index: u32) -> *mut c_void {
    if devices == ptr::null_mut() { return ptr::null_mut(); }

    let devices = unsafe { &mut *(devices as *mut Carbons) };
    match devices.get_mut(index as usize) {
        Some(device) => device as *mut _ as *mut c_void,
        None => ptr::null_mut(),
    }
}

/// Given a handle to a Carbon device, returns a handle to the inner atmegau2 interface.
///
/// This function does not take ownership of the Carbon device passed to it, so the caller is
/// responsible for managing its memory.
///
/// The returned handle to the atmegau2 interface is heap-allocated and must be freed with
/// `lf_release`.
///
/// The returned handle to the atmegau2 interface is valid only as long as its parent Carbon device
/// is valid, which is in turn only valid for as long as the containing Carbon devices list is.
/// In other words, for a device list `devices`, once `lf_release(devices)` has been called,
/// the interface handle that was returned from this function will be invalid.
#[no_mangle]
pub extern "C" fn lf_carbon_atmegau2(carbon: *mut c_void) -> *mut c_void {
    if carbon == ptr::null_mut() { return ptr::null_mut(); }
    let carbon = unsafe { &mut *(carbon as *mut Carbon) };
    let atmegau2 = carbon.atmegau2() as *mut LfDevice;
    let boxed_atmegau2 = Box::new(atmegau2);
    Box::into_raw(boxed_atmegau2) as *mut c_void
}

/// Given a handle to a Carbon device, returns a handle to the inner atsam4s interface.
///
/// This function does not take ownership of the Carbon device passed to it, so the caller is
/// responsible for managing its memory.
///
/// The returned handle to the atsam4s interface is heap-allocated and must be freed with
/// `lf_release`.
///
/// The returned handle to the atsam4s interface is valid only as long as its parent Carbon device
/// is valid, which is in turn only valid for as long as the containing Carbon devices list is.
/// In other words, for a device list `devices`, once `lf_release(devices)` has been called,
/// the interface handle that was returned from this function will be invalid.
#[no_mangle]
pub extern "C" fn lf_carbon_atsam4s(carbon: *mut c_void) -> *mut c_void {
    if carbon == ptr::null_mut() { return ptr::null_mut(); }
    let carbon = unsafe { &mut *(carbon as *mut Carbon) };
    let atsam4s = carbon.atsam4s() as *mut LfDevice;
    let boxed_atsam4s = Box::new(atsam4s);
    Box::into_raw(boxed_atsam4s) as *mut c_void
}

/// Releases the memory used by the resource.
///
/// This function takes ownership of the resource pointer, then frees the backing memory. The
/// pointer should never be accessed after calling this function.
///
/// If `NULL` is passed to this function, then no action will be taken, and `LfResult::NullPointer`
/// will be returned.
pub extern "C" fn lf_release(resource: *mut c_void) -> LfResult {
    if resource == ptr::null_mut() { return LfResult::NullPointer; }

    // Re-create the Box<_> from the raw pointer.
    let boxed: Box<_> = unsafe { Box::from_raw(resource as *mut _) };
    // Explicitly drop the box
    drop(boxed);
    LfResult::Success
}

/// Creates an empty argument list to be used with `lf_invoke`.
///
/// This function creates an opaque, heap-allocated struct used for preparing a remote function
/// call to Flipper. The typical usage is to create the argument list, then to append each argument
/// to it using `lf_append_arg`, then to pass it to `lf_invoke` to perform the invocation.
///
/// Since the list is heap-allocated, it is the responsibility of the caller to free the memory
/// when the list is no longer needed. The proper way to do this is by using `lf_release`.
#[no_mangle]
pub extern "C" fn lf_create_args(argv: *mut *mut c_void) -> LfResult {
    // Create a vector with enough capacity for all of the arguments
    let mut args: Args = Args::new();

    // Box the vector and return the raw heap pointer to the caller
    let ptr = Box::into_raw(Box::new(args));
    unsafe { *argv = ptr as *mut c_void };

    LfResult::Success
}

/// Appends a new argument (value and type) onto an existing argument list.
///
/// If the argument being appended is smaller than 8 bytes, then its value should be initialized
/// using its native type initialization, then cast into a `LfValue` when passed to this function.
///
/// ```
/// void *argv;
/// lf_create_args(&argv);
///
/// uint32_t argument1 = 0x40000000;
/// LfType arg1kind = lf_uint32;
///
/// lf_append_arg(argv, (LfValue) argument1, arg1kind);
/// ```
///
/// As new items are appended to the list, the list will automatically re-alloc itself and grow as
/// necessary.
///
/// If the value passed for `kind` is not valid (i.e. not defined in the LfType enum), then
/// nothing will be appended to the list, and `LfResult::IllegalType` will be returned.
#[no_mangle]
pub extern "C" fn lf_append_arg(argv: *mut c_void, value: LfValue, kind: LfType) -> LfResult {
    if argv == ptr::null_mut() { return LfResult::NullPointer; }

    // Reconstruct the args vector from the argv handle
    let mut args: Box<Args> = unsafe { Box::from_raw(argv as *mut _) };

    let _ = match kind {
        LfType::lf_uint8 => args.append(value as u8),
        LfType::lf_uint16 => args.append(value as u16),
        LfType::lf_uint32 => args.append(value as u32),
        LfType::lf_uint64 => args.append(value as u64),
        _ => return LfResult::IllegalType,
    };

    // Forget the args box so it doesn't get freed
    mem::forget(args);
    LfResult::Success
}

/// Executes a remote function on the given Flipper device.
///
/// Flipper invocations are composed of 4 things:
///
///   1) The name of the module where the function to execute is defined
///   2) The index of the function to execute within its parent module
///   3) The list of argument values and types to be passed to the function
///   4) The expected return type that should be produced by executing the function
///
/// To send an invocation, we must also provide the handle of the device to send to, and the
/// address of a variable to store the return value.
///
/// # Example
///
/// Consider the built-in "led" module, which controls Flipper's onboard RGB led. The primary
/// function in this module is `led_rgb(uint8_t red, uint8_t green, uint8_t blue)`, which is the
/// first function in the module (located at index 0). The led module is controlled by the
/// `atmegau2` processor on Flipper: Carbon edition.
///
/// In order to invoke `led_rgb(10, 20, 30)` in C, we would do the following:
///
/// ```
/// // Get the list of Flipper: Carbon USB devices
/// void *devices;
/// uint32_t length;
/// lf_attach_carbons_usb(&devices, &length);
///
/// // Select the first Carbon device in the list
/// void *carbon = lf_select_carbon(devices, 0);
///
/// // Get the atmegau2 interface for the selected Carbon device
/// void *atmegau2 = lf_carbon_atmegau2(carbon);
///
/// // Construct the argument list
/// void *args;
/// lf_create_args(&args);
///
/// uint8_t red = 10, green = 20, blue = 30;
/// lf_append_arg(args, (LfValue) red, lf_uint8);
/// lf_append_arg(args, (LfValue) green, lf_uint8);
/// lf_append_arg(args, (LfValue) blue, lf_uint8);
///
/// // Send the invocation and read the result
/// LfValue result;
/// lf_invoke(atmegau2, "led", 0, args, lf_void, &result);
///
/// // Release the argument list and carbon list
/// lf_release(args);
/// lf_release(devices);
/// ```
#[no_mangle]
pub extern "C" fn lf_invoke(
    device: *mut c_void,
    module: *const c_char,
    function: LfFunction,
    argv: *const c_void,
    return_type: LfType,
    return_value: *mut LfValue,
) -> LfResult {
    if device == ptr::null_mut() { return LfResult::NullPointer; }
    if module == ptr::null() { return LfResult::NullPointer; }
    if argv == ptr::null() { return LfResult::NullPointer; }

    // The return_value pointer should not be null unless the return type is void
    if return_value == ptr::null_mut() {
        match return_type {
            LfType::lf_void => (),
            _ => return LfResult::NullPointer,
        }
    }

    // Reconstruct the device trait object from the raw pointer given
    let mut device: Box<Box<dyn LfDevice>> = unsafe { Box::from_raw(device as *mut _) };

    // Build a Rust string from the given char *.
    let module_cstr = unsafe { CStr::from_ptr(module) };
    let module_string = match module_cstr.to_str() {
        Ok(module_string) => module_string,
        Err(_) => return LfResult::InvalidString,
    };

    // Reconstruct the args box from the raw pointer given
    let args: Box<Args> = unsafe { Box::from_raw(argv as *mut _) };

    // Perform the invocation and
    match device.invoke(module_string, function, return_type, &args) {
        Some(ret) => unsafe {
            match return_type {
                LfType::lf_uint8 => *(return_value as *mut u8) = ret as u8,
                LfType::lf_uint16 => *(return_value as *mut u16) = ret as u16,
                LfType::lf_uint32 => *(return_value as *mut u32) = ret as u32,
                LfType::lf_uint64 => *(return_value as *mut u64) = ret as u64,
                LfType::lf_int8 => *(return_value as *mut i8) = ret as i8,
                LfType::lf_int16 => *(return_value as *mut i16) = ret as i16,
                LfType::lf_int32 => *(return_value as *mut i32) = ret as i32,
                LfType::lf_int64 => *(return_value as *mut i64) = ret as i64,
                _ => (),
            }
        }
        None => return LfResult::InvocationError,
    }

    // We only borrowed these structs, so we should not drop the boxes when this function ends
    mem::forget(device);
    mem::forget(args);

    LfResult::Success
}