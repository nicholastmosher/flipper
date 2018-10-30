use std::io::{Read, Write, Result};
use crate::runtime::{LfDevice, Args};
use crate::runtime::protocol::LfType;

pub enum UartBaud {
    FMR,
    DFU,
}

impl UartBaud {
    fn to_baud(&self) -> u8 {
        match *self {
            UartBaud::FMR => 0x00,
            UartBaud::DFU => 0x08,
        }
    }
}

pub struct Uart0<'a, T: LfDevice> {
    device: &'a mut T,
}

impl<'a, T: LfDevice> Uart0<'a, T> {
    pub fn new(device: &'a mut T) -> Self {
        Uart0 { device }
    }

    /// Configures the Uart0 module with a given baud rate and
    /// interrupts enabled flag.
    pub fn configure(&mut self, baud: UartBaud, interrupts: bool) {
        let mut args = Args::new();
        args.append(baud.to_baud())
            .append(if interrupts { 1u8 } else { 0u8 });
        self.device.invoke("uart0", 0, LfType::lf_void, &args);
    }

    /// Indicates whether the Uart0 bus is ready to read or write.
    pub fn ready(&mut self) -> bool {
        let args = Args::new();
        let ret: u8 = self.device.invoke("uart0", 1, LfType::lf_uint8, &args).unwrap() as u8;
        ret != 0
    }
}

impl<'a, T: LfDevice> Write for Uart0<'a, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let device_buffer = self.device.malloc(buf.len() as u32).expect("should malloc");
        self.device.push(device_buffer, buf);
        let mut args = Args::new();
        args.append(device_buffer)
            .append(buf.len() as u32);
        self.device.invoke("uart0", 2, LfType::lf_void, &args);
        self.device.free(device_buffer);
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl<'a, T: LfDevice> Read for Uart0<'a, T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() == 0 { return Ok(0) }
        let device_buffer = self.device.malloc(buf.len() as u32).expect("should malloc");
        let mut args = Args::new();
        args.append(device_buffer)
            .append(buf.len() as u32);
        self.device.invoke("uart0", 3, LfType::lf_void, &args);
        self.device.free(device_buffer);
        Ok(buf.len())
    }
}