//! Protocol Data Units.
//!
//! This is your entry point when you're writing a custom transport:
//!
//! - [`Request`] serializes the request into a proper PDU.
//! - [`Response`] deserializes PDU into the structure.

use bytes::{Buf, BufMut};

use crate::protocol::{Encode, Error, Exception, Function, bytes::Decode};

/// Request Protocol Data Unit.
#[derive(Copy, Clone)]
pub struct Request<T> {
    /// Modbus function code.
    pub function_code: u8,

    /// Function-dependent arguments that follow the function code.
    pub args: T,
}

impl<T> Request<T> {
    /// Wrap the function arguments into PDU.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fennec_modbus::protocol::{
    ///     Encode,
    ///     data_unit::Request,
    ///     function::{
    ///         ReadRegisters,
    ///         read_registers::{Args, Holding},
    ///     },
    /// };
    ///
    /// let data_unit = Request::wrap::<ReadRegisters<Holding, u16>>(Args::new(107, 3)?);
    ///
    /// assert_eq!(
    ///     data_unit.encode_into_bytes(),
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

impl<T: Encode> Encode for Request<T> {
    fn encode_into(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.function_code);
        self.args.encode_into(buf);
    }
}

/// Response Protocol Data Unit.
#[derive(Copy, Clone)]
pub enum Response<F: Function> {
    /// Successful response.
    Ok(F::Decode),

    /// The connection is healthy, but the response is a Modbus exception.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fennec_modbus::protocol::{
    ///     Decode,
    ///     Exception,
    ///     data_unit::Response,
    ///     function::{ReadRegisters, read_registers::Holding},
    /// };
    ///
    /// let mut buf: &[u8] = &[
    ///     0x83, // function code + error flag
    ///     0x04, // server device failure
    /// ];
    /// let response = Response::<ReadRegisters<Holding, u16>>::decode_from(&mut buf)?;
    /// assert!(matches!(response, Response::Exception(Exception::ServerDeviceFailure)));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    ///
    /// # Handling unknown error code
    ///
    /// ```rust
    /// use fennec_modbus::protocol::{
    ///     Decode,
    ///     Exception,
    ///     data_unit::Response,
    ///     function::{ReadRegisters, read_registers::Holding},
    /// };
    ///
    /// let mut buf: &[u8] = &[
    ///     0x83, // function code + error flag
    ///     0xFF, // unknown error code
    /// ];
    /// let response = Response::<ReadRegisters<Holding, u16>>::decode_from(&mut buf)?;
    /// assert!(matches!(response, Response::Exception(Exception::Custom(0xFF))));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    Exception(Exception),
}

impl<F> Decode for Response<F>
where
    F: Function,
    F::Decode: Decode,
{
    type Output = Result<<F::Decode as Decode>::Output, Exception>;

    fn decode_from(buf: &mut impl Buf) -> Result<Self::Output, Error> {
        match buf.try_get_u8()? {
            function_code if function_code == F::CODE => Ok(Ok(F::Decode::decode_from(buf)?)),
            function_code if function_code == (F::CODE | 0x80) => {
                Ok(Err(Exception::decode_from(buf)?))
            }
            function_code => Err(Error::UnexpectedFunctionCode(function_code)),
        }
    }
}
