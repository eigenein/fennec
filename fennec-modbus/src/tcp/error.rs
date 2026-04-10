use alloc::string::{String, ToString};

use thiserror::Error;

use crate::protocol;

#[must_use]
#[derive(Debug, Error)]
pub enum Error {
    #[error("protocol error")]
    Protocol(#[from] protocol::Error),

    #[error("invalid unit ID ({0})")]
    InvalidUnitId(u8),

    #[error("payload size exceeded ({0} bytes)")]
    PayloadSizeExceeded(usize),

    #[error("payload format error: {0}")]
    PayloadFormat(String),
}

impl From<binrw::Error> for Error {
    fn from(error: binrw::Error) -> Self {
        Self::PayloadFormat(error.to_string())
    }
}
