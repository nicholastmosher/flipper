use flipper::{Client, LfType, Args, Flipper};
use libusb::Context;

trait Gpio: Client {
    fn gpio_configure(&mut self) {
        self.invoke("gpio", 3, LfType::lf_void, &Args::new());
    }

    fn gpio_enable(&mut self, enabled: u32, disabled: u32) {
        let mut args = Args::new();
        args.append(enabled)
            .append(disabled);
        self.invoke("gpio", 2, LfType::lf_void, &args);
    }

    fn gpio_write(&mut self, high: u32, low: u32) {
        let mut args = Args::new();
        args.append(high)
            .append(low);
        self.invoke("gpio", 1, LfType::lf_void, &args);
    }

    fn gpio_read(&mut self, pins: u32) -> u32 {
        let mut args = Args::new();
        args.append(pins);
        self.invoke("gpio", 0, LfType::lf_uint32, &args)
            .expect("should get read result") as u32
    }
}

impl<T> Gpio for T where T: Client { }

fn main() {
    let mut context = Context::new().expect("should get usb context");
    let mut flippers = Flipper::attach_usb(&mut context);
    let flipper = flippers.first_mut().expect("should find one Flipper");

    flipper.gpio_write(1 << 22, 0);
}