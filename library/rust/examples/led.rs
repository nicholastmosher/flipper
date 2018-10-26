use libusb::Context;
use flipper_core::{
    lf::Args,
    fmr::LfDevice,
    fmr::LfType,
    carbon::Carbon,
};

struct Led<'a, T: LfDevice> {
    device: &'a mut T,
}

impl<'a, T: LfDevice> Led<'a, T> {
    pub fn new(device: &'a mut T) -> Led<'a, T> {
        Led { device }
    }

    pub fn rgb(&mut self, red: u8, green: u8, blue: u8) {
        let args = Args::new()
            .append(red)
            .append(green)
            .append(blue);
        let module: &str = "len";
        println!("{}", module.len());
        self.device.invoke(module, 0, LfType::void, args);
    }
}

fn main() {
    let mut context = Context::new().expect("should get libusb context");
    let mut carbons = Carbon::attach(&mut context);
    let carbon = carbons.first_mut().expect("should get a Flipper on usb");

    let mut led = Led::new(carbon.atmegau2());
    led.rgb(10, 10, 0);
}