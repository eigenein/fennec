use alloc::string::{String, ToString};

use thiserror::Error;

use crate::protocol;

#[derive(Debug, Error)]
pub enum Error {
    #[error("protocol error")]
    Protocol(#[source] protocol::Error),

    #[error("payload size exceeded ({0} bytes)")]
    PayloadSizeExceeded(usize),

    #[error("payload format error: {0}")]
    PayloadFormat(String),

    #[error("payload size mismatch (expected {n_expected_bytes} bytes, got {n_actual_bytes})")]
    PayloadSizeMismatch { n_expected_bytes: usize, n_actual_bytes: usize },
}

impl From<binrw::Error> for Error {
    fn from(error: binrw::Error) -> Self {
        Self::PayloadFormat(error.to_string())
    }
}
