#![no_std]
#![doc = include_str!("../README.md")]

extern crate alloc;

pub mod contrib;
mod error;
pub mod protocol;
pub mod tcp;

pub use self::error::Error;
