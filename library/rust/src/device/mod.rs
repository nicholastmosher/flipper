mod usb;
mod atsam;

pub use self::usb::UsbDevice;
pub use self::usb::get_usb_devices;
pub use self::atsam::AtsamDevice;
