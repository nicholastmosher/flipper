//! Provides utilities for defining and executing remote calls using FMR.
//!
//! The Flipper Message Runtime (FMR) is the mechanism that Flipper uses to
//! remotely execute functions on the Flipper device from a Host machine such
//! as a desktop computer or phone.
//!
//! These utilities lay the groundwork for users to create bindings to custom
//! Flipper "modules" (not to be confused with rust modules).

use libc;
use libc::{c_void, c_int, c_char};
use std::mem;
use std::ptr;
use std::ffi::CString;

use crate::_lf_device;

use crate::runtime::*;
use crate::runtime::protocol::*;

/// Types transmitted over FMR are encoded in a u8.
type _lf_type = libc::uint8_t;

/// Values transmitted over FMR are packaged in a u64.
type _lf_value = libc::uint64_t;

/// Function indices are represented by a u8.
type _lf_index = libc::uint8_t;

/// The address type for a pointer on Flipper.
type _lf_4s_address = libc::uint32_t;

// The concrete encodings for types in libflipper.
const LF_TYPE_U8: _lf_type = 0;
const LF_TYPE_U16: _lf_type = 1;
const LF_TYPE_VOID: _lf_type = 2;
const LF_TYPE_U32: _lf_type = 3;
const LF_TYPE_PTR: _lf_type = 6;
const LF_TYPE_U64: _lf_type = 7;

/// The internal `libflipper` representation of a function argument.
/// This is used for FFI when we ask libflipper to execute a function
/// on a device.
#[derive(Debug, Eq, PartialEq)]
#[repr(C, packed)]
struct _lf_arg {
    arg_type: _lf_type,
    arg_value: _lf_value,
}

/// The libflipper native representation of a linked list. We need this
/// representation so we can construct parameter lists for FMR invocations.
#[repr(C)]
pub(crate) struct _lf_ll {
    item: *const _lf_arg,
    destructor: *const c_void,
    next: *const _lf_ll,
}

mod libflipper {
    use super::*;
    #[link(name = "flipper")]
    extern {
        pub(crate) fn lf_get_selected() -> *const _lf_device;
        pub(crate) fn lf_ll_append(ll: *mut *mut _lf_ll, item: *const c_void, destructor: *const c_void) -> c_int;
        pub(crate) fn lf_invoke(device: *const _lf_device, module: *const c_char, function: _lf_index, ret_type: u8, ret_val: *const _lf_value, args: *const _lf_ll) -> i32;
        pub(crate) fn lf_push(device: *const _lf_device, destination: _lf_4s_address, source: *const c_void, length: u32) -> i32;
        pub(crate) fn lf_pull(device: *const _lf_device, destination: *const c_void, source: _lf_4s_address, length: u32) -> i32;
        pub(crate) fn lf_malloc(device: *const _lf_device, size: u32, ptr: *mut _lf_4s_address) -> i32;
        pub(crate) fn lf_free(device: *const _lf_device, ptr: _lf_4s_address) -> i32;
    }
}


pub fn current_device() -> *const _lf_device {
    unsafe {
        libflipper::lf_get_selected()
    }
}

/// Invokes a remote function call to a Flipper device.
///
/// Consider the following C function, which belongs to a Flipper module.
///
/// ```c
/// uint8_t foo(uint16_t bar, uint32_t baz, uint64_t qux);
/// ```
///
/// To execute this function using `invoke` would look like this:
///
/// ```rust,no_run
/// use flipper::{
///     lf,
///     Flipper,
/// };
///
/// let args = lf::Args::new()
///                .append(10 as u16)  // bar
///                .append(20 as u32)  // baz
///                .append(30 as u64); // qux
///
/// let flipper = Flipper::attach().expect("should attach to Flipper");
/// let output: u8 = lf::invoke(&flipper, "my_module_name", 0, args);
/// ```
pub fn invoke<R: LfReturnable, T: LfDevice>(device: &mut T, module: &str, index: u8, args: Args) -> R {
    unsafe {
        let mut arglist: *mut _lf_ll = ptr::null_mut();
        for arg in args.iter() {
            libflipper::lf_ll_append(&mut arglist, &arg.0 as *const LfArg as *const c_void, ptr::null());
        }
        let module_name = CString::new(module).unwrap();
        let mut ret: _lf_value = mem::uninitialized();
        libflipper::lf_invoke(libflipper::lf_get_selected(), module_name.as_ptr(), index, R::lf_type() as u8, &mut ret as *mut _lf_value, arglist);
        R::from(LfReturn(ret))
    }
}

/// Pushes a buffer of data to an address on the given Flipper device.
///
/// This function will push all of the data in the `data` buffer. If less
/// data should be pushed, simply pass a subslice of the buffer where the
/// data is coming from.
pub fn push<T: LfDevice>(device: &mut T, destination: &LfPointer, data: &[u8]) {
    unsafe {
        libflipper::lf_push(libflipper::lf_get_selected(), destination.0, data.as_ptr() as *const c_void, data.len() as u32);
    }
}

/// Pulls data from a source address on the given Flipper device.
///
/// This function will continue polling for data until enough bytes
/// are received to fill the entire destination buffer. Use the slice
/// operation to make the destination slice the exact size of the data
/// needed.
pub fn pull<T: LfDevice>(device: &mut T, destination: &mut [u8], src: &LfPointer) {
    unsafe {
        libflipper::lf_pull(libflipper::lf_get_selected(), destination.as_mut_ptr() as *mut c_void, src.0, destination.len() as u32);
    }
}

/// Allocates the given number of bytes on the given Flipper device.
///
/// The returned value is a pointer in the Flipper device address space, and should not
/// be used as a native pointer on the host machine. The Flipper address is intended to
/// be used with other low-level Flipper functions (e.g. `lf::push` and `lf::pull`).
pub fn malloc<T: LfDevice>(device: &mut T, size: u32) -> LfPointer {
    unsafe {
        let mut address: _lf_4s_address = mem::uninitialized();
        libflipper::lf_malloc(libflipper::lf_get_selected(), size, &mut address);
        LfPointer(address)
    }
}

pub fn free<T: LfDevice>(device: &mut T, memory: LfPointer) {
    unsafe {
        libflipper::lf_free(libflipper::lf_get_selected(), memory.0 as _lf_4s_address);
    }
}

mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_arg() {
        let argu8 = Arg::from(123u8);
        let argu8_native = argu8.0;
        assert_eq!(argu8_native.arg_type, LF_TYPE_U8);
        assert_eq!(argu8_native.arg_value, 123u64);

        let argu16 = Arg::from(234u16);
        let argu16_native = argu16.0;
        assert_eq!(argu16_native.arg_type, LF_TYPE_U16);
        assert_eq!(argu16_native.arg_value, 234u64);

        let argu32 = Arg::from(345u32);
        let argu32_native = argu32.0;
        assert_eq!(argu32_native.arg_type, LF_TYPE_U32);
        assert_eq!(argu32_native.arg_value, 345u64);
    }

    #[test]
    fn test_arg_builder() {
        let args = Args::new()
            .append(1u8)
            .append(2u16)
            .append(3u32)
            .append(4u8)
            .append(5u16);

        let expected = vec![
            _lf_arg { arg_type: LF_TYPE_U8, arg_value: 1u64 },
            _lf_arg { arg_type: LF_TYPE_U16, arg_value: 2u64 },
            _lf_arg { arg_type: LF_TYPE_U32, arg_value: 3u64 },
            _lf_arg { arg_type: LF_TYPE_U8, arg_value: 4u64 },
            _lf_arg { arg_type: LF_TYPE_U16, arg_value: 5u64 },
        ];

        for (actual, expected) in args.iter().zip(expected) {
            assert_eq!(actual.0, expected);
        }
    }
}
