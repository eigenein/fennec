use binrw::BinRead;

use crate::pdu::{exception, function};

/// Top-level response protocol data unit.
///
/// This is a «peek and dispatch» glue for [`Response`].
#[derive(Debug, BinRead)]
#[br(big)]
pub struct Payload {
    /// Original request function code – optionally, summed with the exception flag.
    #[br(restore_position)]
    #[expect(dead_code)]
    pub function_code: u8,

    #[br(args(function_code))]
    pub response: Response,
}

/// Response payload: either a successful functional response or an exception.
///
/// This requires that the function code is already read and stashed to the arguments.
#[derive(Debug, BinRead)]
#[br(big, import(function_code: u8))]
pub enum Response {
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
        let response: Response = Payload::read(&mut Cursor::new(RESPONSE)).unwrap().into();
        assert!(
            matches!(
                response,
                Response::Exception(exception::Response {
                    original_function_code: 3,
                    error: FunctionalError::Server(ServerError::ServerDeviceFailure),
                }),
            ),
            "actual response: {response:?}",
        );
    }
}
