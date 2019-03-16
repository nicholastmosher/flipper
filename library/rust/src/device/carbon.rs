use std::io::{self as io, Read, Write};

use crate::{Client, LfType, Args};
use crate::runtime::Modules;
use crate::error::Result;
use crate::device::{
    AtsamClient,
    UsbClient,
};

lazy_static! {
    static ref ATMEGA_MODULES: Vec<&'static str> = vec![
        "led",
    ];
}

pub struct Carbon<'a> {
    modules: Modules,
    atmegau2: UsbClient<'a>,
    atsam4s: Option<AtsamClient<'a, UsbClient<'a>>>,
}

impl<'a> Carbon<'a> {
    pub fn new(atmegau2: UsbClient<'a>) -> Carbon<'a> {
        let mut carbon = Carbon {
            modules: Modules::new(),
            atsam4s: None,
            atmegau2,
        };

        // Erase the lifetime of the UsbDevice. Since AtsamDevice requires a reference
        // to the UsbDevice, placing them both in the same struct creates a self-referential
        // problem. However, since neither the UsbDevice nor the AtsamDevice can be individually
        // moved out of this struct, they will both be dropped at the same time.
        let atmegau2 = unsafe { &mut *(&mut carbon.atmegau2 as *mut _) };
        let atsam4s = AtsamClient::new(atmegau2);
        carbon.atsam4s = Some(atsam4s);
        carbon
    }

    fn atmegau2(&mut self) -> &mut UsbClient<'a> {
        &mut self.atmegau2
    }

    fn atsam4s(&mut self) -> &mut AtsamClient<'a, UsbClient<'a>> {
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

impl<'a> Drop for Carbon<'a> {
    fn drop(&mut self) {
        // Drop the AtsamDevice before dropping the UsbDevice
        self.atsam4s = None
    }
}
