use thiserror::Error;

use crate::protocol;

#[must_use]
#[derive(Debug, Error)]
pub enum Error {
    #[error("Modbus protocol error")]
    Protocol(#[from] protocol::Error),

    #[error("invalid unit ID ({0})")]
    InvalidUnitId(u8),

    #[error("payload size exceeded ({0} bytes)")]
    PayloadSizeExceeded(usize),
}
