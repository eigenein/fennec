#![no_std]

extern crate alloc;

pub mod pdu;
pub mod tcp;

use alloc::string::{String, ToString};

use thiserror::Error;

pub type Result<T = (), E = Error> = core::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid count requested ({0})")]
    InvalidCount(usize),

    #[error("invalid length requested ({0})")]
    InvalidLength(usize),

    #[error("payload size mismatch (expected {n_expected_bytes} bytes, got {n_actual_bytes})")]
    PayloadSizeMismatch { n_expected_bytes: usize, n_actual_bytes: usize },

    #[error("I/O error: {0}")]
    IoError(String),
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
