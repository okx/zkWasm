#![deny(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![feature(thread_local)]

pub mod circuits;
pub mod cli;
pub mod foreign;
pub mod runtime;
pub mod traits;

#[cfg(feature = "checksum")]
pub mod image_hasher;

mod profile;

#[cfg(test)]
pub mod test;

#[macro_use]
extern crate lazy_static;
extern crate downcast_rs;
