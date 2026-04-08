use binrw::BinRead;

use crate::pdu::exception;

/// Response protocol data unit.
#[derive(Debug, BinRead)]
#[br(big)]
pub enum Response<T: for<'a> BinRead<Args<'a> = ()>> {
    Ok(T),
    Exception(exception::Response),
}

#[cfg(test)]
mod tests {
    use binrw::io::Cursor;

    use super::*;
    use crate::pdu::{
        exception::{FunctionalError, ServerError},
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
                    error: FunctionalError::Server(ServerError::ServerDeviceFailure),
                    ..
                }),
            ),
            "actual response: {response:?}",
        );
    }
}
