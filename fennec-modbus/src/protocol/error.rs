use alloc::string::{String, ToString};

use thiserror::Error;

/// Low-level protocol representation error.
#[derive(Debug, Error)]
pub enum WireError {
    #[error("invalid count requested ({0})")]
    InvalidCount(usize),

    #[error("bad binary format: {0}")]
    BadFormat(String),

    #[error("coil number mismatch (expected {n_expected_bytes} bytes, got {n_actual_bytes})")]
    CoilNumberMismatch { n_expected_bytes: usize, n_actual_bytes: usize },
}

impl From<binrw::Error> for WireError {
    fn from(error: binrw::Error) -> Self {
        Self::BadFormat(error.to_string())
    }
}
