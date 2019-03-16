#[macro_use]
extern crate flipper;

use libusb::Context;
use flipper::Flipper;

flipper_module!(led::Led: [
    0 => fn led_rgb(red: u8, green: u8, blue: u8) -> LfType::lf_void,
]);

fn main() {
    use led::Led;

    let mut context = Context::new().expect("should get usb context");
    let mut flippers = Flipper::attach_usb(&mut context);
    let flipper = flippers.first_mut().expect("should find one flipper");

    let _ = flipper.led_rgb(10, 05, 10);
}