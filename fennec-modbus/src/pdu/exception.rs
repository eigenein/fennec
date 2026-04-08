use binrw::BinRead;
use thiserror::Error;

#[derive(BinRead)]
#[br(big)]
pub struct Response {
    #[br(map = |it: u8| it & 0x7F)]
    pub original_function_code: u8,

    pub error: FunctionalError,
}

/// The server received the request without a communication error, but could not handle it.
#[repr(u8)]
#[derive(Copy, Clone, Debug, BinRead, Error)]
#[br(big, repr = u8)]
pub enum FunctionalError {
    /// The function code received in the query is not an allowable action for the server:
    ///
    /// - the function was not implemented in the unit selected;
    /// - the server is in the wrong state to process a request of this type.
    #[error("illegal function")]
    IllegalFunction = 0x01,

    /// The data address received in the query is not an allowable address for the server.
    ///
    /// The combination of reference number and transfer length is invalid.
    #[error("illegal data address")]
    IllegalDataAddress = 0x02,

    /// A value contained in the query data field is not an allowable value for server.
    #[error("illegal data value")]
    IllegalDataValue = 0x03,

    /// An unrecoverable error occurred while the server was attempting to perform the requested action.
    #[error("server device failure")]
    ServerDeviceFailure = 0x04,

    /// The server has accepted the request and is
    /// processing it, but a long duration of time will be
    /// required to do so. This response is returned to
    /// prevent a timeout error from occurring in the
    /// client. The client can next issue a «Poll Program
    /// Complete» message to determine if processing is
    /// completed.
    #[error("acknowledge")]
    Acknowledge = 0x05,

    /// The server is engaged in processing a long–duration program command.
    /// The client should retransmit the message later when the server is free.
    #[error("server device busy")]
    ServerDeviceBusy = 0x06,

    /// The server attempted to read record file, but  detected a parity error in the memory.
    /// The client can retry the request, but service may be required on the server device.
    #[error("memory parity error")]
    MemoryParityError = 0x08,

    /// The gateway was unable to allocate an internal communication path from the input port
    /// to the output port for processing the request.
    #[error("gateway path unavailable")]
    GatewayPathUnavailable = 0x0A,

    /// No response was obtained from the target device.
    ///
    /// Usually means that the device is not present on the network.
    #[error("gateway target device failed to respond")]
    GatewayTargetDeviceFailedToRespond = 0x0B,
}
