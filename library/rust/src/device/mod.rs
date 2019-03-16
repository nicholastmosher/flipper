use libusb::Context;

mod usb;
mod atsam;
pub mod carbon;

pub use self::usb::UsbClient;
pub use self::atsam::AtsamClient;
pub use self::carbon::Carbon;
use self::usb::get_usb_devices;

use std::io::{Read, Write};
use crate::{Client, LfType, Args};
use crate::error::Result;
use crate::runtime::{
    Modules,
    protocol::{LfFunction, LfPointer}
};

pub struct Flipper<'a> {
    inner: Box<Client + 'a>,
    modules: Modules,
}

impl<'a> Flipper<'a> {
    pub fn attach_usb(context: &mut Context) -> Vec<Flipper> {
        get_usb_devices(context).into_iter()
            .map(|usb| Flipper::new(Carbon::new(usb)))
            .collect()
    }

    fn new<T: Client + 'a, I: Into<Box<T>>>(inner: I) -> Flipper<'a> {
        Flipper { inner: inner.into(), modules: Modules::new() }
    }
}

impl<'a> Client for Flipper<'a> {
    fn modules(&mut self) -> &mut Modules {
        &mut self.modules
    }

    fn reader(&mut self) -> &mut Read {
        self.inner.reader()
    }

    fn writer(&mut self) -> &mut Write {
        self.inner.writer()
    }

    fn invoke(&mut self, module: &str, function: LfFunction, ret: LfType, args: &Args) -> Result<u64> {
        self.inner.invoke(module, function, ret, args)
    }

    fn load(&mut self, module: &str) -> Result<u64> {
        self.inner.load(module)
    }

    fn push(&mut self, pointer: LfPointer, data: &[u8]) -> Result<()> {
        self.inner.push(pointer, data)
    }

    fn pull(&mut self, pointer: LfPointer, buffer: &mut [u8]) -> Result<()> {
        self.inner.pull(pointer, buffer)
    }

    fn malloc(&mut self, size: u32) -> Result<LfPointer> {
        self.inner.malloc(size)
    }

    fn free(&mut self, pointer: LfPointer) -> Result<()> {
        self.inner.free(pointer)
    }
}