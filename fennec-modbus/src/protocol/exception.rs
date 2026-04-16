use core::fmt::Debug;

use bytes::Buf;
use thiserror::Error;

use crate::{protocol, protocol::bytes::Decode};

/// High-level protocol error.
///
/// The server received the request without a communication error, but could not handle it.
#[must_use]
#[derive(Copy, Clone, Debug, Error)]
pub enum Exception {
    /// The function code received in the query is not an allowable action for the server:
    ///
    /// - the function was not implemented in the unit selected;
    /// - the server is in the wrong state to process a request of this type.
    #[error("illegal function")]
    IllegalFunction,

    /// The data address received in the query is not an allowable address for the server.
    ///
    /// The combination of reference number and transfer length is invalid.
    #[error("illegal data address")]
    IllegalDataAddress,

    /// A value contained in the query data field is not an allowable value for server.
    #[error("illegal data value")]
    IllegalDataValue,

    /// An unrecoverable error occurred while the server was attempting to perform the requested action.
    #[error("server device failure")]
    ServerDeviceFailure,

    /// The server has accepted the request and is processing it, but a long duration of time will be
    /// required to do so.
    ///
    /// This response is returned to prevent a timeout error from occurring in the client.
    /// The client can next issue a «Poll Program Complete» message to determine if processing is completed.
    #[error("acknowledge")]
    Acknowledge,

    /// The server is engaged in processing a long–duration program command.
    ///
    /// The client should retransmit the message later when the server is free.
    #[error("server device busy")]
    ServerDeviceBusy,

    /// The server attempted to read record file, but  detected a parity error in the memory.
    ///
    /// The client can retry the request, but service may be required on the server device.
    #[error("memory parity error")]
    MemoryParityError,

    /// The gateway was unable to allocate an internal communication path from the input port
    /// to the output port for processing the request.
    #[error("gateway path unavailable")]
    GatewayPathUnavailable,

    /// No response was obtained from the target device.
    ///
    /// Usually means that the device is not present on the network.
    #[error("gateway target device failed to respond")]
    GatewayTargetDeviceFailedToRespond,

    /// Non-standard error code.
    #[error("custom error ({0})")]
    Custom(u8),
}

impl Decode for Exception {
    fn decode_from(buf: &mut impl Buf) -> Result<Self, protocol::Error> {
        match buf.try_get_u8()? {
            0x01 => Ok(Self::IllegalFunction),
            0x02 => Ok(Self::IllegalDataAddress),
            0x03 => Ok(Self::IllegalDataValue),
            0x04 => Ok(Self::ServerDeviceFailure),
            0x05 => Ok(Self::Acknowledge),
            0x06 => Ok(Self::ServerDeviceBusy),
            0x08 => Ok(Self::MemoryParityError),
            0x0A => Ok(Self::GatewayPathUnavailable),
            0x0B => Ok(Self::GatewayTargetDeviceFailedToRespond),
            function_code => Ok(Self::Custom(function_code)),
        }
    }
}
