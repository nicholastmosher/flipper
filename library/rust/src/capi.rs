use std::mem;
use std::ptr;
use std::ffi::CStr;
use std::os::raw::{c_void, c_char};

use crate::runtime::{Client, Args};
use crate::runtime::protocol::*;
use crate::device::Flipper;

use std::marker::PhantomPinned;
use libusb::Context;
use std::pin::Pin;

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
    lf_illegal_handle = 8,
}

struct UsbDevices<'a> {
    usb_context: Context,
    usb_flippers: Vec<Flipper<'a>>,
    _pin: PhantomPinned,
}

impl<'a> UsbDevices<'a> {
    fn new(usb_context: Context) -> Pin<Box<UsbDevices<'a>>> {
        let devices = UsbDevices {
            usb_context,
            usb_flippers: vec![],
            _pin: PhantomPinned,
        };
        let mut boxed: Pin<Box<UsbDevices>> = Box::pin(devices);

        unsafe {
            let mut_ref: Pin<&mut UsbDevices> = Pin::as_mut(&mut *(&mut boxed as *mut _));
            let usbDevices = Pin::get_unchecked_mut(mut_ref);
            let flippers = Flipper::attach_usb(&mut usbDevices.usb_context);
            usbDevices.usb_flippers.extend(flippers);
        }

        boxed
    }
}

enum FFIContainer<'a> {
    Flipper(&'a mut Client),
    UsbDevices(Pin<Box<UsbDevices<'a>>>),
    ArgsList(Args),
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
/// `lf_release(devices)`.
#[no_mangle]
pub extern "C" fn lf_attach_usb(devices: *mut *mut c_void, length: *mut u32) -> LfResult {
    let usb_context = Context::new().expect("should get usb context");
    let usb_devices = UsbDevices::new(usb_context);
    let usb_count = usb_devices.usb_flippers.len();
    let ffi_container = Box::new(FFIContainer::UsbDevices(usb_devices));
    let ffi_pointer = Box::into_raw(ffi_container);

    unsafe {
        *devices = ffi_pointer as *mut c_void;
        *length = usb_count as u32;
    }

    LfResult::lf_success
}

/// Retrieves a device from the device list at the given index. Index 0 is the
/// first device.
///
/// The returned handle represents a single attached Flipper device. This
/// handle is only valid while the device list it came from is valid. That is,
/// if `lf_release(devices)` is called, then the Flipper handle that was
/// returned by this function is no longer valid (but still must be freed).
/// Handles returned by `lf_select` must be freed using `lf_release`.
///
/// If the given devices pointer is NULL, then NULL is returned.
///
/// If the given index is out of bounds for the device list, then NULL is
/// returned.
#[no_mangle]
pub extern "C" fn lf_select(devices: *mut c_void, index: u32, device: *mut *mut c_void) -> LfResult {
    if devices == ptr::null_mut() { return LfResult::lf_null_pointer; }
    let mut ffi_devices_container: Box<FFIContainer> = unsafe { Box::from_raw(devices as *mut _) };

    match *ffi_devices_container {
        FFIContainer::UsbDevices(ref mut devices) => {
            let client: Option<&mut Client> = unsafe {
                let mut_ref = Pin::get_unchecked_mut(devices.as_mut());
                mut_ref.usb_flippers
                    .get_mut(index as usize)
                    .map(|device| device as &mut dyn Client)
            };

            let ffi_pointer: *mut c_void = client
                .map(|client| FFIContainer::Flipper(client))
                .map(|ffi_container| Box::new(ffi_container))
                .map(|boxed| Box::into_raw(boxed) as *mut c_void)
                .unwrap_or(ptr::null_mut());

            unsafe { *device = ffi_pointer }
        }
        _ => return LfResult::lf_illegal_handle
    }

    mem::forget(ffi_devices_container);
    LfResult::lf_success
}

/// Adds a new argument (value and type) to an argument list.
///
/// The argument list is represented by an opaque pointer. A new argument
/// list can be created by passing a reference to a NULL pointer as the
/// `argv` argument. Then, the value and type parameters will be appended to
/// the new list, or to the existing list.
///
/// ```c
/// void *argv = NULL;
/// lf_append_arg(&argv, (LfValue) 0x20000000, lf_uint32);
/// lf_append_arg(&argv, (LfValue) 0x40000000, lf_uint32);
/// lf_append_arg(&argv, (LfValue) 0x80000000, lf_uint32);
///
/// // Make sure to release the list when you're done
/// lf_release(argv);
/// ```
///
/// If the argument being appended is smaller than 8 bytes, then its value
/// should be initialized using its native type initialization, then cast into
/// a `LfValue` when passed to this function.
///
/// As new items are appended to the list, the list will automatically re-alloc
/// itself and grow as necessary.
///
/// If the value passed for `kind` is not valid (i.e. not defined in the LfType
/// enum), then nothing will be appended to the list, and an `LfResult` of
/// `lf_illegal_type` will be returned.
#[no_mangle]
pub extern "C" fn lf_append_arg(argv: *mut *mut c_void, value: LfValue, kind: LfType) -> LfResult {
    if argv == ptr::null_mut() { return LfResult::lf_null_pointer; }

    // If the argv handle is null, create a new, empty args list and assign it
    unsafe {
        if *argv == ptr::null_mut() {
            let args = Args::new();
            let ffi_container = Box::new(FFIContainer::ArgsList(args));
            let ffi_pointer = Box::into_raw(ffi_container) as *mut c_void;
            *argv = ffi_pointer
        }
    }

    // Reconstruct the args vector from the argv handle
    let mut ffi: Box<FFIContainer> = unsafe { Box::from_raw(*argv as *mut _) };

    match &mut *ffi {
        FFIContainer::ArgsList(ref mut args) => {
            match kind {
                LfType::lf_uint8 => args.append(value as u8),
                LfType::lf_uint16 => args.append(value as u16),
                LfType::lf_uint32 => args.append(value as u32),
                LfType::lf_uint64 => args.append(value as u64),
                _ => return LfResult::lf_illegal_type,
            };
        }
        _ => return LfResult::lf_illegal_handle,
    }

    // Forget the ffi box so it doesn't get freed
    mem::forget(ffi);
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
/// lf_release(args);
/// lf_release(flipper);
/// lf_release(usb_devices);
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
    let mut ffi_device_container: Box<FFIContainer> = unsafe { Box::from_raw(device as *mut _) };
    let device = match *ffi_device_container {
        FFIContainer::Flipper(ref mut client) => client,
        _ => return LfResult::lf_illegal_handle,
    };

    // Build a Rust string from the given char *.
    let module_cstr = unsafe { CStr::from_ptr(module) };
    let module_string = match module_cstr.to_str() {
        Ok(module_string) => module_string,
        Err(_) => return LfResult::lf_invalid_string,
    };

    // Reconstruct the args box from the raw pointer given
    let ffi_args_container: Box<FFIContainer> = unsafe { Box::from_raw(argv as *mut _) };
    let args = match *ffi_args_container {
        FFIContainer::ArgsList(ref args) => args,
        _ => return LfResult::lf_illegal_handle,
    };

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
    mem::forget(ffi_device_container);
    mem::forget(ffi_args_container);

    LfResult::lf_success
}

#[no_mangle]
pub extern "C" fn lf_release(argv: *mut c_void) -> LfResult {
    if argv == ptr::null_mut() { return LfResult::lf_null_pointer; }
    let boxed: Box<FFIContainer> = unsafe { Box::from_raw(argv as *mut _) };
    drop(boxed);
    LfResult::lf_success
}
