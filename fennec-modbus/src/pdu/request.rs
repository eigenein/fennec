use binrw::BinWrite;

use crate::function;

/// Request protocol data unit.
#[derive(BinWrite)]
pub enum Request {
    ReadHoldingRegisters(function::read_holding_registers::Request),
}
