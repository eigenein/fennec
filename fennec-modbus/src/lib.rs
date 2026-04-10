#![no_std]

extern crate alloc;

pub mod pdu;
pub mod tcp;

use alloc::string::{String, ToString};

use thiserror::Error;

pub type Result<T, E> = core::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("protocol error: {0}")]
    Protocol(#[source] ProtocolError),

    #[error("transport error: {0}")]
    Transport(#[source] TransportError),
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("invalid count requested ({0})")]
    InvalidCount(usize),

    #[error("wire format error: {0}")]
    WireFormat(String),

    #[error("coil number mismatch (expected {n_expected_bytes} bytes, got {n_actual_bytes})")]
    CoilNumberMismatch { n_expected_bytes: usize, n_actual_bytes: usize },
}

impl From<binrw::Error> for ProtocolError {
    fn from(error: binrw::Error) -> Self {
        Self::WireFormat(error.to_string())
    }
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("payload size exceeded ({0} bytes)")]
    PayloadSizeExceeded(usize),

    #[error("payload format error: {0}")]
    PayloadFormat(String),

    #[error("payload size mismatch (expected {n_expected_bytes} bytes, got {n_actual_bytes})")]
    PayloadSizeMismatch { n_expected_bytes: usize, n_actual_bytes: usize },
}

impl From<binrw::Error> for TransportError {
    fn from(error: binrw::Error) -> Self {
        Self::PayloadFormat(error.to_string())
    }
}
