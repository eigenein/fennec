use binrw::{BinRead, BinWrite};

pub trait Request: BinWrite {
    const FUNCTION_CODE: u8;
    type Response: BinRead;
}
