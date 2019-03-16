use flipper::{Client, LfType, Args, Flipper};
use libusb::Context;

struct Gpio<'a, T: Client> {
    device: &'a mut T,
}

impl<'a, T: Client> Gpio<'a, T> {
    pub fn new(device: &'a mut T) -> Gpio<'a, T> { Gpio { device } }

    pub fn configure(&mut self) {
        self.device.invoke("gpio", 3, LfType::lf_void, &Args::new());
    }

    pub fn enable(&mut self, enabled: u32, disabled: u32) {
        let mut args = Args::new();
        args.append(enabled)
            .append(disabled);
        self.device.invoke("gpio", 2, LfType::lf_void, &args);
    }

    pub fn write(&mut self, high: u32, low: u32) {
        let mut args = Args::new();
        args.append(high)
            .append(low);
        self.device.invoke("gpio", 1, LfType::lf_void, &args);
    }

    pub fn read(&mut self, pins: u32) -> u32 {
        let mut args = Args::new();
        args.append(pins);
        self.device.invoke("gpio", 0, LfType::lf_uint32, &args)
            .expect("should get read result") as u32
    }
}

fn main() {

    let mut context = Context::new().expect("should get usb context");
    let mut flippers = Flipper::attach_usb(&mut context);
    let flipper = flippers.first_mut().expect("should find one Flipper");

    let mut gpio = Gpio::new(flipper);

    let result = gpio.read(0);
    println!("Result is {}", result);
}