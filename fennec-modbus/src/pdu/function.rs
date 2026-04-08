use alloc::vec::Vec;

use binrw::BinRead;

pub mod read_holding_registers;

/// Successful function response.
#[derive(Debug, BinRead)]
#[br(big)]
pub enum Response {
    ReadHoldingRegisters(read_holding_registers::Response),

    UserDefined {
        function_code: u8,

        #[br(parse_with = binrw::helpers::until_eof)]
        payload: Vec<u8>,
    },
}

#[cfg(test)]
mod tests {
    use binrw::io::Cursor;

    use super::*;

    #[test]
    fn user_defined_function_ok() {
        const RESPONSE: &[u8] = &[
            100, // user-defined function code
            1, 2, 3, // payload
        ];
        let response = Response::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert!(matches!(
            response,
            Response::UserDefined { function_code: 100, payload } if payload == [1, 2, 3]
        ));
    }
}
