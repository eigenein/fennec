use alloc::string::{String, ToString};

use thiserror::Error;

use crate::protocol::Exception;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid count requested ({0})")]
    InvalidCount(usize),

    #[error("wire format error: {0}")]
    WireFormat(String),

    #[error("coil number mismatch (expected {n_expected_bytes} bytes, got {n_actual_bytes})")]
    CoilNumberMismatch { n_expected_bytes: usize, n_actual_bytes: usize },

    #[error("exception")]
    Exception(#[from] Exception),
}

impl From<binrw::Error> for Error {
    fn from(error: binrw::Error) -> Self {
        Self::WireFormat(error.to_string())
    }
}
