use flipper::{Client, LfType, Args, Flipper};
use libusb::Context;

trait Led: Client {
    fn led_rgb(&mut self, red: u8, green: u8, blue: u8) {
        let mut args = Args::new();
        args.append(red)
            .append(green)
            .append(blue);
        self.invoke("led", 0, LfType::lf_void, &args);
    }
}

impl<T> Led for T where T: Client { }

fn main() {
    let mut context = Context::new().expect("should get usb context");

    let mut flippers = Flipper::attach_usb(&mut context);
    let flipper = flippers.first_mut().expect("should find one flipper");

    flipper.led_rgb(10, 05, 10);
}