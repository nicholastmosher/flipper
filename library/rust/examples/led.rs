use flipper_core::{
    LfDevice,
    LfType,
    lf::Args,
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
        self.device.invoke("led", 0, LfType::void, args);
    }
}

fn main() {
    let mut carbons = Carbon::attach();
    let carbon = carbons.iter_mut().next().expect("should get a Flipper on usb");

    let mut led = Led::new(carbon.atmegau2());
    led.rgb(10, 20, 10);
}