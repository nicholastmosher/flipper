#[macro_use]
extern crate flipper;

use libusb::Context;
use flipper::Flipper;

flipper_module!(gpio::Gpio: [
    3 => fn gpio_configure() -> LfType::lf_void,
    2 => fn gpio_enable(enabled: u32, disabled: u32) -> LfType::lf_void,
    1 => fn gpio_write(high: u32, low: u32) -> LfType::lf_void,
    0 => fn gpio_read(pins: u32) -> LfType::lf_uint32,
]);

fn main() {
    use gpio::Gpio;

    let mut context = Context::new().expect("should get usb context");
    let mut flippers = Flipper::attach_usb(&mut context);
    let flipper = flippers.first_mut().expect("should find one Flipper");

    let _ = flipper.gpio_write(1 << 22, 0);
}