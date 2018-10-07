use libusb::Context;
use crate::device::{
    AtsamDevice,
    UsbDevice,
    get_usb_devices,
};

pub struct Carbon<'a> {
    atmegau2: UsbDevice<'a>,
}

impl<'a> Carbon<'a> {
    pub fn attach(context: &'a mut Context) -> Vec<Carbon<'a>> {
        get_usb_devices(context)
            .into_iter()
            .map(|atmegau2| Carbon { atmegau2 })
            .collect()
    }

    pub fn atmegau2(&mut self) -> &mut UsbDevice<'a> {
        &mut self.atmegau2
    }

    pub fn atsam4s(&'a mut self) -> AtsamDevice<'a, UsbDevice<'a>> {
        AtsamDevice::new(&mut self.atmegau2)
    }
}
