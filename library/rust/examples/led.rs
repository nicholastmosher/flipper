use flipper::{
    Client,
    LfType,
    Args,
    Carbon,
};

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
    let mut carbons = Carbon::attach_usb();
    let carbon = carbons.iter_mut().next().expect("should get a Flipper on usb");

    let mut led = Led::new(carbon);
    led.rgb(10, 05, 10);
}