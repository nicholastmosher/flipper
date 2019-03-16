use std::mem;
use std::ptr;
use std::os::raw::{c_void, c_char};
use flipper::capi::LfResult;

#[link(name = "flipper")]
extern {
    fn lf_attach_usb(devices: *mut *mut c_void, length: *mut u32) -> u32;
    fn lf_select(devices: *mut c_void, index: u32, device: *mut *mut c_void) -> LfResult;
    fn lf_create_args(argv: *mut *mut c_void) -> u32;
    fn lf_append_arg(argv: *mut c_void, value: u64, kind: u8) -> u32;
    fn lf_invoke(device: *mut c_void, module: *const c_char, function: u8, args: *const c_void, ret: u8, value: *mut u64) -> u32;
    fn lf_release(resource: *mut c_void) -> u32;
}

unsafe fn led_rgb(flipper: *mut c_void, red: u8, green: u8, blue: u8) {
    let mut argv: *mut c_void = mem::uninitialized();
    lf_create_args(&mut argv as *mut *mut c_void);
    lf_append_arg(argv, red as u64, 0);
    lf_append_arg(argv, green as u64, 0);
    lf_append_arg(argv, blue as u64, 0);
    lf_invoke(flipper, b"led".as_ptr() as *const i8, 0, argv, 2, ptr::null_mut());
    lf_release(argv);
}

fn main() {
    unsafe {
        let mut devices: *mut c_void = mem::uninitialized();
        let mut flipper: *mut c_void = mem::uninitialized();
        let mut length: u32 = 0;

        lf_attach_usb(&mut devices as *mut *mut c_void, &mut length as *mut u32);
        lf_select(devices, 0, &mut flipper);

        for r in 0..20 {
            for g in 0..20 {
                for b in 0..20 {
                    led_rgb(flipper, r * 3, g * 3, b * 3);
                }
            }
        }

        lf_release(flipper);
        lf_release(devices);
    }
}