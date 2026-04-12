use binrw::BinRead;

use crate::protocol::{Error, exception, r#struct::Readable};

/// Response protocol data unit.
#[derive(Clone, derive_more::Debug, derive_more::Unwrap, derive_more::TryUnwrap, BinRead)]
#[br(big)]
pub enum Response<T: Readable> {
    Ok(T),
    Exception(exception::Response),
}

impl<T: Readable> Response<T> {
    pub fn into_result(self) -> Result<T, Error> {
        match self {
            Self::Ok(response) => Ok(response),
            Self::Exception(response) => Err(Error::Exception(response.exception)),
        }
    }
}

#[cfg(test)]
mod tests {
    use binrw::io::Cursor;

    use super::*;
    use crate::protocol::{
        exception::{Exception, ServerError},
        function::read_holding_registers,
    };

    #[test]
    fn parse_exception_ok() {
        const RESPONSE: &[u8] = &[
            0x83, // function code + error flag
            0x04, // server device failure
        ];
        let response =
            Response::<read_holding_registers::Response>::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert!(
            matches!(
                response,
                Response::Exception(exception::Response {
                    exception: Exception::Server(ServerError::ServerDeviceFailure),
                    ..
                }),
            ),
            "actual response: {response:?}",
        );
    }
}
