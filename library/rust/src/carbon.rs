use std::io::{self as io, Read, Write};
use std::slice::{Iter, IterMut};
use libusb::Context;
use crate::Client;
use crate::runtime::{
    protocol::LfType,
    Modules,
    Args,
};
use crate::device::{
    AtsamDevice,
    UsbDevice,
    get_usb_devices,
};

lazy_static! {
    static ref ATMEGA_MODULES: Vec<&'static str> = vec![
        "led",
    ];
}

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
    modules: Modules,
    atmegau2: Box<UsbDevice<'a>>,
    atsam4s: Option<AtsamDevice<'a, UsbDevice<'a>>>,
}

impl<'a> Carbon<'a> {
    fn new(atmegau2: UsbDevice<'a>) -> Carbon<'a> {
        let mut carbon = Carbon {
            modules: Modules::new(),
            atmegau2: Box::new(atmegau2),
            atsam4s: None
        };

        // Erase the lifetime of the UsbDevice. Since AtsamDevice requires a reference
        // to the UsbDevice, placing them both in the same struct creates a self-referential
        // problem. However, since neither the UsbDevice nor the AtsamDevice can be individually
        // moved out of this struct, they will both be dropped at the same time.
        let atmegau2 = unsafe { &mut *(carbon.atmegau2.as_mut() as *mut _) };
        let atsam4s = AtsamDevice::new(atmegau2);
        carbon.atsam4s = Some(atsam4s);
        carbon
    }

    pub fn attach() -> Carbons<'a> {
        let context = Context::new().expect("should get libusb context");
        let mut carbons = Carbons { context: Box::new(context), devices: Some(vec![]) };

        // Erase the lifetime of the context. We never allow a Carbon device to be moved
        // out of the Carbons struct so we can guarantee that they live exactly as long
        // as the libusb Context.
        let context = unsafe { &mut *(carbons.context.as_mut() as *mut _) };

        for atmegau2 in get_usb_devices(context) {
            carbons.devices.as_mut().unwrap().push(Carbon::new(atmegau2))
        }

        carbons
    }

    fn atmegau2(&mut self) -> &mut UsbDevice<'a> {
        &mut self.atmegau2
    }

    fn atsam4s(&mut self) -> &mut AtsamDevice<'a, UsbDevice<'a>> {
        self.atsam4s.as_mut().unwrap()
    }
}

impl<'a> Read for Carbon<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.atsam4s().read(buf)
    }
}

impl<'a> Write for Carbon<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.atsam4s().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.atsam4s().flush()
    }
}

impl<'a> Client for Carbon<'a> {
    fn modules(&mut self) -> &mut Modules {
        &mut self.modules
    }
    fn invoke(&mut self, module: &str, function: u8, ret: LfType, args: &Args) -> Option<u64> {
        let client: &mut Client = if ATMEGA_MODULES.contains(&module) {
            self.atmegau2()
        } else {
            self.atsam4s()
        };
        client.invoke(module, function, ret, args)
    }
}

impl<'a> Drop for Carbon<'a> {
    fn drop(&mut self) {
        // Drop the AtsamDevice before dropping the UsbDevice
        self.atsam4s = None
    }
}
