use std::mem;
use std::ptr;
use std::ffi::CStr;
use std::os::raw::{c_void, c_char};

use crate::runtime::{Client, Args};
use crate::runtime::protocol::*;
use crate::device::carbon::{Carbon, Carbons};

#[repr(u32)]
pub enum LfResult {
    lf_success = 0,
    lf_null_pointer = 1,
    lf_invalid_string = 2,
    lf_package_not_loaded = 3,
    lf_no_devices_found = 4,
    lf_index_out_of_bounds = 5,
    lf_illegal_type = 6,
    lf_invocation_error = 7,
}

/// Returns an opaque pointer to a list of Flipper devices and the length of
/// the list.
///
/// There are no guarantees about the representation of the device list. The
/// returned value should be used solely as a handle to provide to other
/// functions that accept a Flipper list.
///
/// The pointer returned as `devices` is heap-allocated and owned by the caller.
/// The proper way to release the device list is by using
/// `lf_release_usb(devices)`.
#[no_mangle]
pub extern "C" fn lf_attach_usb(devices: *mut *mut c_void, length: *mut u32) -> LfResult {
    let carbons = Carbon::attach_usb();
    if carbons.len() == 0 { return LfResult::lf_no_devices_found; }

    let carbons = Box::new(Carbon::attach_usb());
    let carbons_length = carbons.len();
    let carbons_pointer = Box::into_raw(carbons);

    unsafe {
        *devices = carbons_pointer as *mut c_void;
        *length = carbons_length as u32;
    }

    LfResult::lf_success
}

/// Retrieves a device from the device list at the given index. Index 0 is the
/// first device.
///
/// The returned handle represents a single attached Flipper device. This
/// handle is only valid while the device list it came from is valid. That is,
/// if `lf_release_usb(devices)` is called, then the Flipper handle that was
/// returned by this function is no longer valid (but still must be freed).
/// Handles returned by `lf_select` must be freed using `lf_release_flipper`.
///
/// If the given devices pointer is NULL, then NULL is returned.
///
/// If the given index is out of bounds for the device list, then NULL is
/// returned.
#[no_mangle]
pub extern "C" fn lf_select(devices: *mut c_void, index: u32) -> *mut c_void {
    if devices == ptr::null_mut() { return ptr::null_mut(); }
    let mut devices: Box<Carbons> = unsafe { Box::from_raw(devices as *mut _) };

    let device: Option<Box<&mut dyn Client>> = devices.get_mut(index as usize)
        .map(|device| Box::new(device as &mut Client));

    let device = match device {
        Some(device) => Box::into_raw(device) as *mut c_void,
        None => ptr::null_mut(),
    };

    mem::forget(devices);
    device
}

/// Creates an empty argument list to be used with `lf_invoke`.
///
/// This function creates an opaque, heap-allocated struct used for preparing a
/// remote function call to Flipper. The typical usage is to create the argument
/// list, then to append each argument to it using `lf_append_arg`, then to pass
/// it to `lf_invoke` to perform the invocation.
///
/// Since the list is heap-allocated, it is the responsibility of the caller to
/// free the memory when the list is no longer needed. The proper way to do this
/// is by using `lf_release_args`.
///
/// # Example
///
/// Here's an example of building an argument list using `lf_create_args` and
/// `lf_append_arg`:
///
/// ```c
/// void *argv = NULL;
/// LfResult result = lf_create_args(&argv);
///
/// // Result will be nonzero if there is an error.
/// if (result) {
///     printf("There was an error creating an argument list!\n");
///     return 1;
/// }
///
/// // Add a uint8_t of value 10 as the first argument. See `lf_append_arg`.
/// lf_append_arg(argv, (LfValue) 10, lf_uint8);
///
/// // Release the argument list when you're done with it.
/// lf_release_args(argv);
/// ```
#[no_mangle]
pub extern "C" fn lf_create_args(argv: *mut *mut c_void) -> LfResult {
    // Create a vector with enough capacity for all of the arguments
    let args: Args = Args::new();

    // Box the vector and return the raw heap pointer to the caller
    let ptr = Box::into_raw(Box::new(args));
    unsafe { *argv = ptr as *mut c_void };

    LfResult::lf_success
}

/// Appends a new argument (value and type) onto an existing argument list.
///
/// If the argument being appended is smaller than 8 bytes, then its value
/// should be initialized using its native type initialization, then cast into
/// a `LfValue` when passed to this function.
///
/// ```c
/// void *argv;
/// lf_create_args(&argv);
///
/// uint32_t argument1 = 0x40000000;
/// LfType arg1kind = lf_uint32;
///
/// lf_append_arg(argv, (LfValue) argument1, arg1kind);
/// ```
///
/// As new items are appended to the list, the list will automatically re-alloc
/// itself and grow as necessary.
///
/// If the value passed for `kind` is not valid (i.e. not defined in the LfType
/// enum), then nothing will be appended to the list, and an `LfResult` of
/// `lf_illegal_type` will be returned.
#[no_mangle]
pub extern "C" fn lf_append_arg(argv: *mut c_void, value: LfValue, kind: LfType) -> LfResult {
    if argv == ptr::null_mut() { return LfResult::lf_null_pointer; }

    // Reconstruct the args vector from the argv handle
    let mut args: Box<Args> = unsafe { Box::from_raw(argv as *mut _) };

    let _ = match kind {
        LfType::lf_uint8 => args.append(value as u8),
        LfType::lf_uint16 => args.append(value as u16),
        LfType::lf_uint32 => args.append(value as u32),
        LfType::lf_uint64 => args.append(value as u64),
        _ => return LfResult::lf_illegal_type,
    };

    // Forget the args box so it doesn't get freed
    mem::forget(args);
    LfResult::lf_success
}

/// Executes a remote function on the given Flipper device.
///
/// Flipper invocations are composed of 4 things:
///
///   1) The name of the module where the function to execute is defined
///   2) The index of the function to execute within its parent module
///   3) The list of argument values and types to be passed to the function
///   4) The expected return type that should be produced by executing the
///      function
///
/// To send an invocation, we must also provide the handle of the device to
/// send to, and the address of a variable to store the return value.
///
/// # Example
///
/// Consider the built-in "led" module, which controls Flipper's onboard RGB
/// led. The primary function in this module is
/// `led_rgb(uint8_t red, uint8_t green, uint8_t blue)`, which is the
/// first function in the module (located at index 0).
///
/// In order to invoke `led_rgb(10, 20, 30)` in C, we would do the following:
///
/// ```c
/// // Get the list of Flipper: Carbon USB devices
/// void *usb_devices;
/// uint32_t length;
/// lf_attach_usb(&usb_devices, &length);
///
/// // Select the first Flipper device in the list
/// void *flipper = lf_select(usb_devices, 0);
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
/// lf_invoke(flipper, "led", 0, args, lf_void, &result);
///
/// // Release the argument list, selected Flipper, and usb list
/// lf_release_args(args);
/// lf_release_flipper(flipper);
/// lf_release_usb(usb_devices);
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
    if device == ptr::null_mut() { return LfResult::lf_null_pointer; }
    if module == ptr::null() { return LfResult::lf_null_pointer; }
    if argv == ptr::null() { return LfResult::lf_null_pointer; }

    // The return_value pointer should not be null unless the return type is void
    if return_value == ptr::null_mut() {
        match return_type {
            LfType::lf_void => (),
            _ => return LfResult::lf_null_pointer,
        }
    }

    // Reconstruct the device trait object from the raw pointer given
    let mut device: Box<Box<dyn Client>> = unsafe { Box::from_raw(device as *mut _) };

    // Build a Rust string from the given char *.
    let module_cstr = unsafe { CStr::from_ptr(module) };
    let module_string = match module_cstr.to_str() {
        Ok(module_string) => module_string,
        Err(_) => return LfResult::lf_invalid_string,
    };

    // Reconstruct the args box from the raw pointer given
    let args: Box<Args> = unsafe { Box::from_raw(argv as *mut _) };

    // Perform the invocation and
    match device.invoke(module_string, function, return_type, &args) {
        Ok(ret) => unsafe {
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
        _ => return LfResult::lf_invocation_error,
    }

    // We only borrowed these structs, so we should not drop the boxes when this function ends
    mem::forget(device);
    mem::forget(args);

    LfResult::lf_success
}

/// Releases the given list of USB devices that was created using
/// `lf_attach_usb`.
///
/// The given list of USB devices will be freed and the USB connections of all
/// of the devices in the list will be closed. Any Flipper devices that were
/// selected from this list will be invalid to use after calling this function.
///
/// This function takes ownership of the resource pointer, then frees the
/// backing memory. The pointer should never be accessed after calling this
/// function.
///
/// If `NULL` is passed to this function, then no action will be taken, and
/// `LfResult` of `lf_null_pointer` will be returned.
#[no_mangle]
pub extern "C" fn lf_release_usb(usb_devices: *mut c_void) -> LfResult {
    if usb_devices == ptr::null_mut() { return LfResult::lf_null_pointer; }

    // Re-create the Box<Carbons> from the raw pointer.
    let boxed: Box<Carbons> = unsafe { Box::from_raw(usb_devices as *mut _) };
    // Explicitly drop the box
    drop(boxed);
    LfResult::lf_success
}

/// Releases the given Flipper device that was returned from `lf_select`.
///
/// Each Flipper handle returned from `lf_select` is heap-allocated, and so must
/// always be manually released using this function. In the case that a Flipper
/// was selected from a USB list, and the USB list is released before the
/// selected Flipper, the Flipper will become invalid and unusable (because its
/// USB interface will be freed), but the Flipper handle itself must still be
/// freed.
///
/// If `NULL` is passed to this function, then no action will be taken, and an
/// `LfResult` of `lf_null_pointer` will be returned.
#[no_mangle]
pub extern "C" fn lf_release_flipper(flipper: *mut c_void) -> LfResult {
    if flipper == ptr::null_mut() { return LfResult::lf_null_pointer; }
    let boxed: Box<Carbon> = unsafe { Box::from_raw(flipper as *mut _) };
    drop(boxed);
    LfResult::lf_success
}

/// Releases the given argument list that was created using `lf_create_args`.
///
/// Argument lists are heap-allocated, and so must be freed using this function.
/// After passing an argument list handle to this function, the handle should
/// never be used again, as it will point to invalid memory.
///
/// If `NULL` is passed to this function, then no action will be taken, and an
/// `LfResult` of `lf_null_pointer` will be returned.
#[no_mangle]
pub extern "C" fn lf_release_args(argv: *mut c_void) -> LfResult {
    if argv == ptr::null_mut() { return LfResult::lf_null_pointer; }
    let boxed: Box<Args> = unsafe { Box::from_raw(argv as *mut _) };
    drop(boxed);
    LfResult::lf_success
}
