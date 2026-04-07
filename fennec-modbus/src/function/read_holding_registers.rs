use alloc::vec::Vec;

use binrw::{BinWrite, binread};

use crate::{function, function::Function};

impl Function for function::ReadHoldingRegisters {
    type Request = Request;
    type Response = Response;
}

#[must_use]
#[derive(Copy, Clone, BinWrite)]
pub struct Request {
    pub starting_address: u16,
    pub count: u16,
}

#[must_use]
#[binread]
#[br(big)]
pub struct Response {
    #[br(temp)]
    byte_count: u8,

    #[br(count = byte_count / 2)]
    pub words: Vec<u16>,
}
