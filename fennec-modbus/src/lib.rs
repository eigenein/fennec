#![no_std]

extern crate alloc;

pub mod pdu;
pub mod tcp;

use alloc::string::{String, ToString};
use core::num::TryFromIntError;

use thiserror::Error;

pub type Result<T = (), E = Error> = core::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("incorrect quantity requested ({0})")]
    InvalidQuantity(u16),

    #[error("payload size mismatch (expected {n_expected_bytes} bytes, got {n_actual_bytes})")]
    PayloadSizeMismatch { n_expected_bytes: u8, n_actual_bytes: usize },

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("failed to convert the integer: {0}")]
    TryFromInt(#[from] TryFromIntError),
}

impl From<binrw::Error> for Error {
    fn from(err: binrw::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl From<binrw::io::Error> for Error {
    fn from(err: binrw::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}
