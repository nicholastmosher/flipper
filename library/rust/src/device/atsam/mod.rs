use std::io::{self as io, Read, Write};
use crate::runtime::{
    Client,
    Modules,
};

pub mod uart0;

use self::uart0::Uart0;

pub struct AtsamClient<'a, T: Client> {
    modules: Modules,
    uart: Uart0<'a, T>,
}

impl<'a, T: Client> Read for AtsamClient<'a, T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        let mut total_read = 0;
        for chunk in buf.chunks_mut(128) {
            let read = self.uart.read(chunk)?;
            total_read += read;
        }
        Ok(total_read)
    }
}

impl<'a, T: Client> Write for AtsamClient<'a, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        let mut total_sent = 0;
        for chunk in buf.chunks(128) {
            let sent = self.uart.write(chunk)?;
            total_sent += sent;
        }
        Ok(total_sent)
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        self.uart.flush()
    }
}

impl<'a, T: Client> Client for AtsamClient<'a, T> {
    fn modules(&mut self) -> &mut Modules {
        &mut self.modules
    }

    fn reader(&mut self) -> &mut Read {
        self
    }

    fn writer(&mut self) -> &mut Write {
        self
    }
}

impl<'a, T: Client> AtsamClient<'a, T> {
    pub fn new(atmegau2: &'a mut T) -> AtsamClient<'a, T> {
        AtsamClient {
            uart: Uart0::new(atmegau2),
            modules: Modules::new(),
        }
    }
}