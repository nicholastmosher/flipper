use std::io::{Read, Write};
use std::pin::Pin;
use std::marker::PhantomPinned;

use crate::{Client, LfType, Args};
use crate::error::Result;
use crate::device::AtsamClient;
use crate::runtime::{
    Modules,
    protocol::{LfFunction, LfPointer}
};

lazy_static! {
    static ref ATMEGA_MODULES: Vec<&'static str> = vec![
        "led",
    ];
}

pub struct Carbon<'a, Atmega: Client> {
    modules: Modules,
    atmegau2: Atmega,
    atsam4s: Option<AtsamClient<'a, Atmega>>,
    _pin: PhantomPinned,
}

impl<'a, Atmega: Client> Carbon<'a, Atmega> {
    pub fn new(atmegau2: Atmega) -> Pin<Box<Carbon<'a, Atmega>>> {
        let carbon = Carbon {
            modules: Modules::new(),
            atmegau2,
            atsam4s: None,
            _pin: PhantomPinned,
        };
        let mut carbon_pin = Box::pin(carbon);

        // Since AtsamDevice requires a reference to the UsbDevice, placing
        // them both in the same struct creates a self-referential problem.
        // Because of this, Carbon::new returns a Pin<Box<Carbon>> so that the
        // Carbon struct will never move, and never invalidate the self references.
        unsafe {
            // Get a mutable reference to the Carbon inside the Pin pointer.
            let carbon = Pin::get_unchecked_mut(carbon_pin.as_mut());

            // Get a reference to the AtmegaU2 inside the Carbon and erase
            // its lifetime. Since Carbon is pinned and neither the atmegau2
            // nor the atsam4s can be moved out of this struct, they will
            // ultimately have the same lifetime.
            let atmegau2 = &mut *(&mut carbon.atmegau2 as *mut _);

            // Create an Atsam4s driver wrapping the AtmegaU2 driver.
            let atsam4s = AtsamClient::new(atmegau2);

            // Insert the Atsam4s into the Carbon device.
            carbon.atsam4s = Some(atsam4s);
        };
        carbon_pin
    }

    fn atmegau2(&mut self) -> &mut Client {
        &mut self.atmegau2
    }

    fn atsam4s(&mut self) -> &mut Client {
        self.atsam4s.as_mut().unwrap()
    }
}

impl<'a, T: Client> Client for Carbon<'a, T> {
    fn modules(&mut self) -> &mut Modules {
        &mut self.modules
    }

    fn reader(&mut self) -> &mut Read {
        self.atsam4s().reader()
    }

    fn writer(&mut self) -> &mut Write {
        self.atsam4s().writer()
    }

    fn invoke(&mut self, module: &str, function: u8, ret: LfType, args: &Args) -> Result<u64> {
        let client: &mut Client = if ATMEGA_MODULES.contains(&module) {
            self.atmegau2()
        } else {
            self.atsam4s()
        };
        client.invoke(module, function, ret, args)
    }
}

impl<'a, T: Client> Client for Pin<Box<Carbon<'a, T>>> {
    fn modules(&mut self) -> &mut Modules {
        let carbon = unsafe { Pin::get_unchecked_mut(self.as_mut()) };
        carbon.modules()
    }

    fn reader(&mut self) -> &mut Read {
        let carbon = unsafe { Pin::get_unchecked_mut(self.as_mut()) };
        carbon.reader()
    }

    fn writer(&mut self) -> &mut Write {
        let carbon = unsafe { Pin::get_unchecked_mut(self.as_mut()) };
        carbon.writer()
    }

    fn invoke(&mut self, module: &str, function: LfFunction, ret: LfType, args: &Args) -> Result<u64> {
        let carbon = unsafe { Pin::get_unchecked_mut(self.as_mut()) };
        carbon.invoke(module, function, ret, args)
    }

    fn load(&mut self, module: &str) -> Result<u64> {
        let carbon = unsafe { Pin::get_unchecked_mut(self.as_mut()) };
        carbon.load(module)
    }

    fn push(&mut self, pointer: LfPointer, data: &[u8]) -> Result<()> {
        let carbon = unsafe { Pin::get_unchecked_mut(self.as_mut()) };
        carbon.push(pointer, data)
    }

    fn pull(&mut self, pointer: LfPointer, buffer: &mut [u8]) -> Result<()> {
        let carbon = unsafe { Pin::get_unchecked_mut(self.as_mut()) };
        carbon.pull(pointer, buffer)
    }

    fn malloc(&mut self, size: u32) -> Result<LfPointer> {
        let carbon = unsafe { Pin::get_unchecked_mut(self.as_mut()) };
        carbon.malloc(size)
    }

    fn free(&mut self, pointer: LfPointer) -> Result<()> {
        let carbon = unsafe { Pin::get_unchecked_mut(self.as_mut()) };
        carbon.free(pointer)
    }
}

impl<'a, T: Client> Drop for Carbon<'a, T> {
    fn drop(&mut self) {
        // Drop the AtsamDevice before dropping the UsbDevice
        self.atsam4s = None
    }
}
