use std::slice::{Iter, IterMut};
use libusb::Context;
use crate::device::{
    AtsamDevice,
    UsbDevice,
    get_usb_devices,
};

pub struct Carbons<'a> {
    context: Box<Context>,
    // Wrap the devices vec in an Option in order to control the drop order.
    // `devices` must be dropped before the context.
    devices: Option<Vec<Carbon<'a>>>,
}

impl<'a> Carbons<'a> {
    pub fn len(&self) -> usize {
        self.devices.as_ref().unwrap().len()
    }

    pub fn get(&self, index: usize) -> Option<&Carbon<'a>> {
        self.devices.as_ref().unwrap().get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Carbon<'a>> {
        self.devices.as_mut().unwrap().get_mut(index)
    }

    pub fn iter(&self) -> Iter<Carbon<'a>> {
        self.devices.as_ref().unwrap().iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<Carbon<'a>> {
        self.devices.as_mut().unwrap().iter_mut()
    }
}

impl<'a> Drop for Carbons<'a> {
    fn drop(&mut self) {
        // Drop the devices vec before dropping the libusb context.
        self.devices = None
    }
}

pub struct Carbon<'a> {
    atmegau2: UsbDevice<'a>,
}

impl<'a> Carbon<'a> {
    pub fn attach() -> Carbons<'a> {
        let mut context = Context::new().expect("should get libusb context");
        let mut carbons = Carbons { context: Box::new(context), devices: Some(vec![]) };

        // Erase the lifetime of the context. We never allow a Carbon device to be moved
        // out of the Carbons struct so we can guarantee that they live exactly as long
        // as the libusb Context.
        let context = unsafe { &mut *(carbons.context.as_mut() as *mut Context) };

        for atmegau2 in get_usb_devices(context) {
            carbons.devices.as_mut().unwrap().push(Carbon { atmegau2 })
        }

        carbons
    }

    pub fn atmegau2(&mut self) -> &mut UsbDevice<'a> {
        &mut self.atmegau2
    }

    pub fn atsam4s(&'a mut self) -> AtsamDevice<'a, UsbDevice<'a>> {
        AtsamDevice::new(&mut self.atmegau2)
    }
}
