use binrw::BinRead;

use crate::pdu::{exception, function};

/// Top-level response protocol data unit.
#[derive(Debug, BinRead)]
#[br(big)]
pub struct Response {
    /// Original request function code – optionally, summed with the exception flag.
    ///
    /// This is a «peek and dispatch» glue for [`Payload`].
    #[br(restore_position)]
    pub function_code: u8,

    #[br(args(function_code))]
    pub payload: Payload,
}

/// Response payload: either a successful functional response or an exception.
#[derive(Debug, BinRead)]
#[br(import(function_code: u8))]
#[br(big)]
pub enum Payload {
    #[br(pre_assert(function_code & 0x80 == 0))]
    Ok(function::Response),

    #[br(pre_assert(function_code & 0x80 != 0))]
    Exception(exception::Response),
}

#[cfg(test)]
mod tests {
    use binrw::io::Cursor;

    use super::*;
    use crate::pdu::exception::{FunctionalError, ServerError};

    #[test]
    fn parse_exception_ok() {
        const RESPONSE: &[u8] = &[
            0x83, // function code + error flag
            0x04, // server device failure
        ];
        let response = Response::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert!(
            matches!(
                response.payload,
                Payload::Exception(exception::Response {
                    original_function_code: 3,
                    error: FunctionalError::Server(ServerError::ServerDeviceFailure),
                }),
            ),
            "actual response: {response:?}",
        );
    }
}
