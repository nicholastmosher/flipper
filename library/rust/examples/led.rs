use flipper::{Client, LfType, Args, Flipper};
use libusb::Context;

struct Led<'a, T: Client> {
    device: &'a mut T,
}

impl<'a, T: Client> Led<'a, T> {
    pub fn new(device: &'a mut T) -> Led<'a, T> {
        Led { device }
    }

    pub fn rgb(&mut self, red: u8, green: u8, blue: u8) {
        let mut args = Args::new();
        args.append(red)
            .append(green)
            .append(blue);
        self.device.invoke("led", 0, LfType::lf_void, &args);
    }
}

fn main() {
    let mut context = Context::new().expect("should get usb context");
    let mut flippers = Flipper::attach_usb(&mut context);
    let flipper = flippers.first_mut().expect("should find one flipper");

    let mut led = Led::new(flipper);
    led.rgb(10, 05, 10);
}