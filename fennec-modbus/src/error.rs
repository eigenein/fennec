use bytes::TryGetError;
use thiserror::Error;

use crate::protocol::Exception;

/// Modbus protocol error.
#[derive(Debug, Error)]
pub enum Error {
    #[error("exception")]
    Exception(#[from] Exception),

    #[error("not enough bytes to read")]
    TryGetError(#[from] TryGetError),

    #[error("unexpected function code ({0})")]
    UnexpectedFunctionCode(u8),

    #[error("payload size exceeded ({0} bytes)")]
    PayloadSizeExceeded(usize),
}
