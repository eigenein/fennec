use alloc::vec::Vec;

use binrw::{BinRead, helpers::until_eof};

use crate::function;

/// Top-level response protocol data unit.
#[derive(BinRead)]
#[br(big)]
pub struct Response {
    /// Response function code.
    ///
    /// It's either the original function code, or
    #[br(restore_position)]
    function_code: u8,

    #[br(args(function_code))]
    payload: Payload,
}

/// Response payload dependent on the error flag.
#[derive(BinRead)]
#[br(import(function_code: u8))]
#[br(big)]
pub enum Payload {
    #[br(pre_assert(function_code & 0x80 == 0))]
    Ok(FunctionResponse),

    #[br(pre_assert(function_code & 0x80 != 0))]
    Error {
        #[br(map = |it: u8| it & 0x7F)]
        original_function_code: u8,
        code: ErrorCode,
    },
}

/// Successful function response.
#[derive(BinRead)]
#[br(big)]
pub enum FunctionResponse {
    ReadHoldingRegisters(function::read_holding_registers::Response),

    /// TODO: missing tests.
    UserDefined {
        code: u8,

        #[br(parse_with = until_eof)]
        payload: Vec<u8>,
    },
}

/// TODO: verify per the specs, section 7.
#[repr(u8)]
#[derive(Copy, Clone, BinRead)]
#[br(big, repr = u8)]
pub enum ErrorCode {
    IllegalFunction = 0x01,
    IllegalDataAddress = 0x02,
    IllegalDataValue = 0x03,
    ServerDeviceFailure = 0x04,
    Acknowledge = 0x05,
    ServerDeviceBusy = 0x06,
    MemoryParityError = 0x08,
    GatewayPathUnavailable = 0x0A,
    GatewayTargetDeviceFailedToRespond = 0x0B,
}
