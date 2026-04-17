use bytes::Buf;

use crate::{
    Error,
    protocol::{Exception, codec},
};

pub struct Decoder;

impl codec::Decoder<Exception> for Decoder {
    fn decode(from: &mut impl Buf) -> Result<Exception, Error> {
        match from.try_get_u8()? {
            0x01 => Ok(Exception::IllegalFunction),
            0x02 => Ok(Exception::IllegalDataAddress),
            0x03 => Ok(Exception::IllegalDataValue),
            0x04 => Ok(Exception::ServerDeviceFailure),
            0x05 => Ok(Exception::Acknowledge),
            0x06 => Ok(Exception::ServerDeviceBusy),
            0x08 => Ok(Exception::MemoryParityError),
            0x0A => Ok(Exception::GatewayPathUnavailable),
            0x0B => Ok(Exception::GatewayTargetDeviceFailedToRespond),
            exception_code => Ok(Exception::Custom(exception_code)),
        }
    }
}
