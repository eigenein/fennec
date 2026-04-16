use bytes::TryGetError;
use thiserror::Error;

use crate::protocol::Exception;

/// Modbus protocol error.
#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid count requested ({0})")]
    InvalidCount(usize),

    #[error("coil number mismatch (expected {n_expected_bytes} bytes, got {n_actual_bytes})")]
    CoilNumberMismatch { n_expected_bytes: usize, n_actual_bytes: usize },

    #[error("exception")]
    Exception(#[from] Exception),

    #[error("not enough bytes to read")]
    TryGetError(#[from] TryGetError),

    #[error("unexpected function code ({0})")]
    UnexpectedFunctionCode(u8),
}
