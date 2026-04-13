//! Protocol Data Units.
//!
//! This is your entry point when you're writing a custom transport:
//!
//! - [`Request`] serializes the request into a proper PDU.
//! - [`Response`] deserializes PDU into the structure.

use binrw::{BinWrite, binread};

use crate::protocol::{Error, Exception, Function, function, r#struct::Writable};

/// Request Protocol Data Unit.
///
/// # Example
///
/// ```rust
/// use fennec_modbus::protocol::{
///     data_unit::Request,
///     function::{ReadHoldingRegisters, read_registers::Args},
///     r#struct::Writable,
/// };
///
/// let args = Args::builder().starting_address(107).n_registers(3).build()?;
/// let data_unit = Request::from_args::<ReadHoldingRegisters>(args);
///
/// assert_eq!(
///     data_unit.to_bytes()?,
///     [
///         0x03, // function code
///         0x00, 0x6B, // starting address: high, low
///         0x00, 0x03, // count: high, low
///     ]
/// );
///
/// # Ok::<_, anyhow::Error>(())
/// ```
#[derive(Copy, Clone, BinWrite)]
#[bw(big)]
pub struct Request<T: Writable> {
    /// Modbus function code.
    pub function_code: function::Code,

    /// Function-dependent arguments that follow the function code.
    pub args: T,
}

impl<T: Writable> Request<T> {
    /// Convert the function arguments into PDU.
    pub const fn from_args<F: Function<Args = T>>(args: T) -> Self {
        Self { function_code: F::CODE, args }
    }
}

/// Response Protocol Data Unit.
#[binread]
#[br(big)]
#[derive(Copy, Clone)]
pub enum Response<F: Function> {
    /// Successful response.
    Ok {
        #[br(temp, assert(function_code == F::CODE))]
        function_code: function::Code,

        /// Function call result.
        output: F::Output,
    },

    /// The connection is healthy, but the response is a Modbus exception.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fennec_modbus::protocol::{
    ///     Exception,
    ///     ServerError,
    ///     data_unit::Response,
    ///     function::ReadHoldingRegisters,
    ///     r#struct::Readable,
    /// };
    ///
    /// let response = Response::<ReadHoldingRegisters>::from_bytes(&[
    ///     0x83, // function code + error flag
    ///     0x04, // server device failure
    /// ])?;
    /// assert!(matches!(
    ///     response,
    ///     Response::Exception { exception: Exception::Server(ServerError::ServerDeviceFailure) }
    /// ));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    ///
    /// # Handling unknown error code
    ///
    /// ```rust
    /// use fennec_modbus::protocol::{
    ///     Exception,
    ///     ServerError,
    ///     data_unit::Response,
    ///     function::ReadHoldingRegisters,
    ///     r#struct::Readable,
    /// };
    ///
    /// let response = Response::<ReadHoldingRegisters>::from_bytes(&[
    ///     0x83, // function code + error flag
    ///     0xFF, // unknown error code
    /// ])?;
    /// assert!(matches!(response, Response::Exception { exception: Exception::Unknown(0xFF) }));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    Exception {
        #[br(temp, assert(function_code == F::CODE.with_error_flag()))]
        function_code: u8,

        /// The error returned by the server.
        exception: Exception,
    },
}

impl<F: Function> Response<F> {
    pub fn into_result(self) -> Result<F::Output, Error> {
        match self {
            Self::Ok { output, .. } => Ok(output),
            Self::Exception { exception, .. } => Err(Error::Exception(exception)),
        }
    }
}
