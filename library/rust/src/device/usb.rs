use std::io::{self as io, Read, Write};
use std::time::Duration;
use crate::runtime::{
    Client,
    Modules,
};

use libusb::{
    self,
    Context, Device, DeviceDescriptor, DeviceHandle
};

const FLIPPER_USB_VENDOR_ID: u16 = 0x16C0;

pub struct UsbDevice<'a> {
    device: Device<'a>,
    handle: DeviceHandle<'a>,
    descriptor: DeviceDescriptor,
    read_endpoint: Endpoint,
    write_endpoint: Endpoint,
    modules: Modules,
}

impl<'a> Read for UsbDevice<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        self.handle.read_bulk(self.read_endpoint.address, buf, Duration::from_secs(1))
            .map_err(|_| io::ErrorKind::Other.into())
    }
}

impl<'a> Write for UsbDevice<'a> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        self.handle.write_bulk(self.write_endpoint.address, buf, Duration::from_secs(1))
            .map_err(|_| io::ErrorKind::Other.into())
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        Ok(())
    }
}

impl<'a> Client for UsbDevice<'a> {
    fn modules(&mut self) -> &mut Modules {
        &mut self.modules
    }
}

pub fn get_usb_devices(context: &mut Context) -> Vec<UsbDevice> {
    let devices = context.devices().expect("should get usb devices");

    let mut usb_devices = vec![];

    // Find all usb devices with Flipper's vendor ID.
    for mut device in devices.iter() {
        let mut descriptor = match device.device_descriptor() {
            Ok(descriptor) => descriptor,
            Err(_) => continue,
        };

        if descriptor.vendor_id() != FLIPPER_USB_VENDOR_ID { continue }

        let handle = match device.open() {
            Ok(handle) => handle,
            Err(_) => continue,
        };

        let read_endpoint = match find_endpoint(
            &mut device,
            &mut descriptor,
            libusb::TransferType::Bulk,
            libusb::Direction::In
        ) {
            Some(endpoint) => endpoint,
            _ => continue,
        };

        let write_endpoint = match find_endpoint(
            &mut device,
            &mut descriptor,
            libusb::TransferType::Bulk,
            libusb::Direction::Out
        ) {
            Some(endpoint) => endpoint,
            _ => continue,
        };

        usb_devices.push(UsbDevice {
            device,
            descriptor,
            handle,
            read_endpoint,
            write_endpoint,
            modules: Modules::new(),
        })
    }

    usb_devices
}

#[derive(Debug)]
struct Endpoint {
    config: u8,
    iface: u8,
    setting: u8,
    address: u8,
}

fn find_endpoint(
    device: &mut Device,
    descriptor: &DeviceDescriptor,
    transfer_type: libusb::TransferType,
    direction: libusb::Direction
) -> Option<Endpoint> {

    for n in 0..descriptor.num_configurations() {
        let config = match device.config_descriptor(n) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for interface in config.interfaces() {
            for interface_descriptor in interface.descriptors() {
                for endpoint_descriptor in interface_descriptor.endpoint_descriptors() {
                    if endpoint_descriptor.direction() == direction
                        && endpoint_descriptor.transfer_type() == transfer_type {
                        return Some(Endpoint {
                            config: config.number(),
                            iface: interface_descriptor.interface_number(),
                            setting: interface_descriptor.setting_number(),
                            address: endpoint_descriptor.address(),
                        });
                    }
                }
            }
        }
    }

    None
}

fn configure_endpoint(handle: &mut DeviceHandle, endpoint: &Endpoint) -> libusb::Result<()> {
    handle.set_active_configuration(endpoint.config)?;
    handle.claim_interface(endpoint.iface)?;
    handle.set_alternate_setting(endpoint.iface, endpoint.setting)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_devices() {
        let mut context = Context::new().expect("should get libusb context");
        let devices = get_usb_devices(&mut context);
        println!("HEllo");
    }
}
