#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![deny(unstable_features)]
#![deny(unused_import_braces)]
#![deny(unused_qualifications)]
//#![deny(warnings)]

#[allow(unused_imports)]
#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
extern crate libc;
extern crate libusb;
#[macro_use]
extern crate lazy_static;

pub mod capi;

#[macro_use]
mod macros;
mod device;
mod runtime;
mod error;

pub use self::runtime::Args;
pub use self::runtime::Client;
pub use self::runtime::protocol::LfType;
pub use self::device::Carbon;

pub use self::error::Result;
pub use failure::Error;
