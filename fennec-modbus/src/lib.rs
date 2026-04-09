#![no_std]

extern crate alloc;

pub mod pdu;
pub mod tcp;

use alloc::string::{String, ToString};
use core::num::TryFromIntError;

use thiserror::Error;

pub type Result<T = ()> = core::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to build the request: {0}")]
    RequestBuilder(#[from] RequestBuilderError),
}

/// Request construction failed before sending it.
#[derive(Debug, Error)]
pub enum RequestBuilderError {
    #[error("incorrect quantity requested ({0})")]
    InvalidQuantity(u16),

    #[error("payload size mismatch (expected {n_expected_bytes} bytes, got {n_actual_bytes})")]
    PayloadSizeMismatch { n_expected_bytes: u8, n_actual_bytes: usize },

    #[error("failed to serialize the payload: {0}")]
    Serialization(String),

    #[error("failed to convert the integer: {0}")]
    TryFromInt(#[from] TryFromIntError),
}

impl From<binrw::Error> for RequestBuilderError {
    fn from(err: binrw::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}
