use thiserror::Error;

use crate::protocol;

/// Modbus-over-TCP error.
#[must_use]
#[derive(Debug, Error)]
pub enum Error {
    /// Protocol-level error occurred.
    #[error("Modbus protocol error")]
    Protocol(#[from] protocol::Error),

    /// Payload size exceeded the maximum for TCP transport.
    #[error("payload size exceeded ({0} bytes)")]
    PayloadSizeExceeded(usize),
}
