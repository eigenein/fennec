//! Protocol Data Units.
//!
//! This is your entry point when you're writing a custom transport:
//!
//! - [`Request`] serializes the request into a proper PDU.
//! - [`Response`] deserializes PDU into the structure.

use deku::{
    DekuContainerWrite,
    DekuError,
    DekuReader,
    DekuWrite,
    no_std_io::{Read, Seek},
    reader::Reader,
};

use crate::protocol::{Error, Exception, Function};

/// Request Protocol Data Unit.
#[derive(Copy, Clone, DekuWrite)]
pub struct Request<T: DekuContainerWrite> {
    /// Modbus function code.
    pub function_code: u8,

    /// Function-dependent arguments that follow the function code.
    pub args: T,
}

impl<T: DekuContainerWrite> Request<T> {
    /// Wrap the function arguments into PDU.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fennec_modbus::protocol::{
    ///     data_unit::Request,
    ///     function::{
    ///         ReadRegisters,
    ///         read_registers::{Args, Holding},
    ///     },
    ///     r#struct::Writable,
    /// };
    ///
    /// let data_unit = Request::wrap::<ReadRegisters<Holding, u16>>(Args::new(107, 3)?);
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
    pub const fn wrap<F: Function<Args = T>>(args: T) -> Self {
        Self { function_code: F::CODE, args }
    }
}

/// Response Protocol Data Unit.
#[derive(Copy, Clone)]
pub enum Response<F: Function> {
    /// Successful response.
    Ok(F::Output),

    /// The connection is healthy, but the response is a Modbus exception.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fennec_modbus::protocol::{
    ///     Exception,
    ///     ServerError,
    ///     data_unit::Response,
    ///     function::{ReadRegisters, read_registers::Holding},
    ///     r#struct::Readable,
    /// };
    ///
    /// let response = Response::<ReadRegisters<Holding, u16>>::from_bytes(&[
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
    ///     function::{ReadRegisters, read_registers::Holding},
    ///     r#struct::Readable,
    /// };
    ///
    /// let response = Response::<ReadRegisters<Holding, u16>>::from_bytes(&[
    ///     0x83, // function code + error flag
    ///     0xFF, // unknown error code
    /// ])?;
    /// assert!(matches!(response, Response::Exception { exception: Exception::Unknown(0xFF) }));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    Exception(Exception),
}

impl<F: Function> Response<F> {
    pub fn into_result(self) -> Result<F::Output, Error> {
        match self {
            Self::Ok(output) => Ok(output),
            Self::Exception(exception) => Err(Error::Exception(exception)),
        }
    }
}

impl<F: Function> DekuReader<'_> for Response<F> {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
        ctx: (),
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        match u8::from_reader_with_ctx(reader, ctx)? {
            function_code if function_code == F::CODE => {
                Ok(Self::Ok(F::Output::from_reader_with_ctx(reader, ctx)?))
            }
            function_code if function_code == (F::CODE | 0x80) => {
                Ok(Self::Exception(Exception::from_reader_with_ctx(reader, ctx)?))
            }
            _ => Err(DekuError::IdVariantNotFound),
        }
    }
}
