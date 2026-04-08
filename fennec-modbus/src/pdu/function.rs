use binrw::{BinRead, BinWrite};

pub mod read_holding_registers;

pub trait Function {
    const CODE: u8;
    type Request: for<'a> BinWrite<Args<'a> = ()> + Send + 'static;
    type Response: for<'a> BinRead<Args<'a> = ()> + Send + 'static;
}
