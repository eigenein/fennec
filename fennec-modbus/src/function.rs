use binrw::{BinRead, BinWrite};

pub mod read_holding_registers;

pub struct Code<const CODE: u8>;

pub trait Function {
    type Request: BinWrite;
    type Response: BinRead;
}

/// Read from 1 to 2000 contiguous status of coils (bits) in a remote device.
pub type ReadCoils = Code<1>;

pub type ReadDiscreteInputs = Code<2>;
pub type ReadHoldingRegisters = Code<3>;
pub type ReadInputRegister = Code<4>;
pub type WriteSingleCoil = Code<5>;
pub type WriteSingleRegister = Code<6>;
pub type ReadExceptionStatus = Code<7>;
pub type Diagnostic = Code<8>;
pub type GetComEventCounter = Code<11>;
pub type GetComEventLog = Code<12>;
pub type WriteMultipleCoils = Code<15>;
pub type WriteMultipleRegisters = Code<16>;
pub type ReportServerId = Code<17>;
pub type ReadFileRecord = Code<20>;
pub type WriteFileRecord = Code<21>;
pub type MaskWriteRegister = Code<22>;
pub type ReadWriteMultipleRegisters = Code<23>;
pub type ReadFifoQueue = Code<24>;
pub type ReadDeviceIdentification = Code<43>;
