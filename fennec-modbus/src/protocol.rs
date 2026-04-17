//! The lowest protocol level.
//!
//! It operates with PDU's and independent of any transport.
//! If you're implementing transport like PDU, you're going to need this module:
//!
//! - **Data units** are the PDU's that you're going to wrap into your transport.
//! - **Functions** are the actual Modbus functions expressed in terms of function code,
//!   request arguments and output.

pub mod address;
pub mod codec;
pub mod exception;
pub mod function;
pub mod request;
pub mod response;

use thiserror::Error;

use crate::protocol::codec::{Decoder, Encoder};

/// Request Protocol Data Unit.
#[derive(Copy, Clone)]
pub struct Request<A> {
    /// Modbus function code.
    pub function_code: u8,

    /// Function-dependent arguments that follow the function code.
    pub args: A,
}

impl<A> Request<A> {
    /// Wrap the function arguments into PDU.
    pub const fn wrap<F: Function<Args = A>>(args: A) -> Self {
        Self { function_code: F::CODE, args }
    }
}

/// Response Protocol Data Unit.
#[derive(Copy, Clone)]
pub enum Response<F: Function> {
    /// Successful response.
    Ok(F::Output),

    /// The connection is healthy, but the response is a Modbus exception.
    Exception(Exception),
}

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

/// Address in a data block.
///
/// That allows constant generic implementations.
pub trait Address {
    /// Concrete address type.
    type Args;

    /// Address encoder.
    type ArgsEncoder: Encoder<Self::Args>;
}

/// Trait that ties function code, arguments and output together.
///
/// Users are free to implement their own functions – be that custom Modbus functions
/// or alternate standard function implementations. In the latter case, consider
/// [making a pull request](https://github.com/eigenein/fennec/pulls).
pub trait Function: function::Code {
    /// Function arguments type.
    type Args;

    /// Function arguments encoder.
    type ArgsEncoder: Encoder<Self::Args>;

    /// Function output type.
    type Output;

    /// Function output decoder.
    type OutputDecoder: Decoder<Self::Output>;
}
