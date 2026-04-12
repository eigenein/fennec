use binrw::{BinWrite, binread};

use crate::protocol::{Error, Exception, Function, r#struct::Writable};

/// Request Protocol Data Unit.
#[derive(Copy, Clone, BinWrite)]
#[bw(big)]
pub struct Request<T: Writable> {
    pub function_code: u8,
    pub args: T,
}

/// Response Protocol Data Unit.
#[binread]
#[br(big)]
#[derive(Copy, Clone)]
pub enum Response<F: Function> {
    Ok {
        #[br(temp, assert(function_code == F::CODE))]
        function_code: u8,

        output: F::Output,
    },
    Exception {
        #[br(temp, assert(function_code == F::CODE | 0x80))]
        function_code: u8,

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ServerError, function::read_holding_registers, r#struct::Readable};

    #[test]
    fn parse_exception_ok() {
        const RESPONSE: &[u8] = &[
            0x83, // function code + error flag
            0x04, // server device failure
        ];
        let response = Response::<read_holding_registers::Function>::from_bytes(RESPONSE).unwrap();
        assert!(matches!(
            response,
            Response::Exception { exception: Exception::Server(ServerError::ServerDeviceFailure) }
        ));
    }

    #[test]
    fn unknown_error_code_ok() {
        const RESPONSE: &[u8] = &[
            0x83, // exception flag and function code
            0xFF, // unknown error code
        ];
        let response = Response::<read_holding_registers::Function>::from_bytes(RESPONSE).unwrap();
        assert!(matches!(response, Response::Exception { exception: Exception::Unknown(0xFF) }));
    }
}
