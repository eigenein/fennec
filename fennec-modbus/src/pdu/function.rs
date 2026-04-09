use binrw::{BinRead, BinWrite};

pub mod read_coils;
pub mod read_discrete_inputs;
pub mod read_holding_registers;
pub mod read_input_registers;
pub mod write_single_coil;

pub trait Function {
    const CODE: u8;
    type Request: for<'a> BinWrite<Args<'a> = ()> + Send + 'static;
    type Response: for<'a> BinRead<Args<'a> = ()> + Send + 'static;
}
