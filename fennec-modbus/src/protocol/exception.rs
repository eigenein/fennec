use core::fmt::Debug;

use binrw::BinRead;
use thiserror::Error;

#[must_use]
#[derive(Copy, Clone, Debug, BinRead)]
#[br(big)]
pub struct Response {
    #[br(assert(function_code & 0x80 != 0, "unexpected function code ({function_code:#X})"))]
    pub function_code: u8,

    pub error: Exception,
}

/// High-level protocol error.
///
/// The server received the request without a communication error, but could not handle it.
#[must_use]
#[derive(Copy, Clone, Debug, BinRead, Error)]
#[br(big)]
pub enum Exception {
    /// Client request was malformed.
    #[error("client error: {0}")]
    Client(ClientError),

    /// The server has failed.
    #[error("server error: {0}")]
    Server(ServerError),

    /// The client should retry.
    #[error("retry: {0}")]
    Retry(RetryException),

    /// The gateway could not delegate the request.
    #[error("gateway error: {0}")]
    Gateway(GatewayError),

    /// Unknown error code.
    #[error("unknown error ({0})")]
    Unknown(u8),
}

#[derive(Copy, Clone, Debug, BinRead, Error)]
#[br(big)]
pub enum ClientError {
    /// The function code received in the query is not an allowable action for the server:
    ///
    /// - the function was not implemented in the unit selected;
    /// - the server is in the wrong state to process a request of this type.
    #[error("illegal function")]
    #[br(magic = 0x01_u8)]
    IllegalFunction,

    /// The data address received in the query is not an allowable address for the server.
    ///
    /// The combination of reference number and transfer length is invalid.
    #[error("illegal data address")]
    #[br(magic = 0x02_u8)]
    IllegalDataAddress,

    /// A value contained in the query data field is not an allowable value for server.
    #[error("illegal data value")]
    #[br(magic = 0x03_u8)]
    IllegalDataValue,
}

#[derive(Copy, Clone, Debug, BinRead, Error)]
#[br(big)]
pub enum ServerError {
    /// An unrecoverable error occurred while the server was attempting to perform the requested action.
    #[error("server device failure")]
    #[br(magic = 0x04_u8)]
    ServerDeviceFailure,

    /// The server attempted to read record file, but  detected a parity error in the memory.
    ///
    /// The client can retry the request, but service may be required on the server device.
    #[error("memory parity error")]
    #[br(magic = 0x08_u8)]
    MemoryParityError,
}

#[derive(Copy, Clone, Debug, BinRead, Error)]
#[br(big)]
pub enum RetryException {
    /// The server has accepted the request and is processing it, but a long duration of time will be
    /// required to do so.
    ///
    /// This response is returned to prevent a timeout error from occurring in the client.
    /// The client can next issue a «Poll Program Complete» message to determine if processing is completed.
    #[error("acknowledge")]
    #[br(magic = 0x05_u8)]
    Acknowledge,

    /// The server is engaged in processing a long–duration program command.
    ///
    /// The client should retransmit the message later when the server is free.
    #[error("server device busy")]
    #[br(magic = 0x06_u8)]
    ServerDeviceBusy,
}

#[derive(Copy, Clone, Debug, BinRead, Error)]
#[br(big)]
pub enum GatewayError {
    /// The gateway was unable to allocate an internal communication path from the input port
    /// to the output port for processing the request.
    #[error("gateway path unavailable")]
    #[br(magic = 0x0A_u8)]
    GatewayPathUnavailable,

    /// No response was obtained from the target device.
    ///
    /// Usually means that the device is not present on the network.
    #[error("gateway target device failed to respond")]
    #[br(magic = 0x0B_u8)]
    GatewayTargetDeviceFailedToRespond,
}

#[cfg(test)]
mod tests {
    use binrw::io::Cursor;

    use super::*;

    #[test]
    fn unknown_error_code_ok() {
        const RESPONSE: &[u8] = &[
            0x83, // exception flag and function code
            0xFF, // unknown error code
        ];
        let response = Response::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert!(matches!(response.error, Exception::Unknown(0xFF)));
    }
}
