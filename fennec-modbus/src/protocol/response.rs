use core::marker::PhantomData;

use bytes::Buf;

use crate::{
    Error,
    protocol::{Function, codec, exception},
};

/// Response decoder.
pub struct Decoder<F>(
    /// Binding to the function type.
    PhantomData<F>,
);

impl<F: Function> codec::Decoder<F::Output> for Decoder<F> {
    fn decode(from: &mut impl Buf) -> Result<F::Output, Error> {
        match from.try_get_u8()? {
            function_code if function_code == F::CODE => F::OutputDecoder::decode(from),
            function_code if function_code == (F::CODE | 0x80) => {
                Err(Error::Exception(exception::Decoder::decode(from)?))
            }
            function_code => Err(Error::UnexpectedFunctionCode(function_code)),
        }
    }
}
