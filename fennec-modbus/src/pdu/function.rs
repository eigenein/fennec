use alloc::vec::Vec;

use binrw::BinRead;

pub mod read_holding_registers;

/// Successful function response.
#[derive(Debug, BinRead)]
#[br(big)]
pub enum Response {
    ReadHoldingRegisters(read_holding_registers::Response),

    /// TODO: missing tests.
    UserDefined {
        code: u8,

        #[br(parse_with = binrw::helpers::until_eof)]
        payload: Vec<u8>,
    },
}
