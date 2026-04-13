//! Implementations of Modbus function arguments and outputs.

use core::marker::PhantomData;

use binrw::{BinRead, BinWrite};

use crate::{protocol, protocol::r#struct::Readable};

pub mod read_coils;
pub mod read_discrete_inputs;
pub mod read_exception_status;
pub mod read_registers;
pub mod write_multiple_coils;
pub mod write_multiple_registers;
pub mod write_single_coil;
pub mod write_single_register;

/// Modbus function codes.
#[derive(Copy, Clone, Debug, Eq, PartialEq, BinRead, BinWrite)]
#[repr(u8)]
#[brw(repr = u8)]
#[must_use]
pub enum Code {
    ReadCoils = 1,
    ReadDiscreteInputs = 2,
    ReadHoldingRegisters = 3,
    ReadInputRegisters = 4,
    WriteSingleCoil = 5,
    WriteSingleRegister = 6,
    ReadExceptionStatus = 7,
    Diagnostics = 8,
    GetCommunicationEventCounter = 11,
    GetCommunicationEventLog = 12,
    WriteMultipleCoils = 15,
    WriteMultipleRegisters = 16,
    ReportServerId = 17,
    ReadFileRecord = 20,
    WriteFileRecord = 21,
    MaskWriteRegister = 22,
    ReadWriteMultipleRegisters = 23,
    ReadFifoQueue = 24,
    EncapsulatedInterfaceTransport = 43,
}

impl Code {
    #[must_use]
    pub const fn with_error_flag(self) -> u8 {
        self as u8 | 0x80
    }
}

/// Read from 1 to 2000 contiguous status of coils in a remote device.
#[derive(Copy, Clone)]
#[must_use]
pub struct ReadCoils<S: Readable>(PhantomData<S>);

impl<S: Readable> protocol::Function for ReadCoils<S> {
    const CODE: Code = Code::ReadCoils;
    type Args = read_coils::Args;
    type Output = read_coils::Output<S>;
}

/// Read from 1 to 2000 contiguous status of discrete inputs in a remote device.
#[derive(Copy, Clone)]
#[must_use]
pub struct ReadDiscreteInputs<S>(PhantomData<S>);

impl<S: Readable> protocol::Function for ReadDiscreteInputs<S> {
    const CODE: Code = Code::ReadDiscreteInputs;
    type Args = read_discrete_inputs::Args;
    type Output = read_discrete_inputs::Output<S>;
}

/// Read the contents of a contiguous block of holding registers in a remote device.
#[must_use]
#[derive(Copy, Clone)]
pub struct ReadHoldingRegisters<V>(PhantomData<V>);

impl<V: read_registers::Value> protocol::Function for ReadHoldingRegisters<V> {
    const CODE: Code = Code::ReadHoldingRegisters;
    type Args = read_registers::Args;
    type Output = read_registers::Output<V>;
}

/// Read the contents of a contiguous block of holding registers in a remote device.
///
/// This is the same function as [`ReadHoldingRegisters`] – but with the register count known at compile time.
#[must_use]
#[derive(Copy, Clone)]
pub struct ReadHoldingRegistersExact<const N: usize, V>(PhantomData<V>);

impl<const N: usize, V: read_registers::Value> protocol::Function
    for ReadHoldingRegistersExact<N, V>
{
    const CODE: Code = Code::ReadHoldingRegisters;
    type Args = read_registers::Args;
    type Output = read_registers::OutputExact<N, V>;
}

/// Read from 1 to 125 contiguous input registers in a remote device.
#[must_use]
#[derive(Copy, Clone)]
pub struct ReadInputRegisters<V>(PhantomData<V>);

impl<V: read_registers::Value> protocol::Function for ReadInputRegisters<V> {
    const CODE: Code = Code::ReadInputRegisters;
    type Args = read_registers::Args;
    type Output = read_registers::Output<V>;
}

/// Read from 1 to 125 contiguous input registers in a remote device.
///
/// This is the same function as [`ReadInputRegisters`] – but with the register count known at compile time.
#[must_use]
#[derive(Copy, Clone)]
pub struct ReadInputRegistersExact<const N: usize, V>(PhantomData<V>);

impl<const N: usize, V: read_registers::Value> protocol::Function
    for ReadInputRegistersExact<N, V>
{
    const CODE: Code = Code::ReadInputRegisters;
    type Args = read_registers::Args;
    type Output = read_registers::OutputExact<N, V>;
}

/// Write a single output to either «on» or «off» in a remote device.
#[must_use]
pub struct WriteSingleCoil;

impl protocol::Function for WriteSingleCoil {
    const CODE: Code = Code::WriteSingleCoil;
    type Args = write_single_coil::Payload;
    type Output = write_single_coil::Payload;
}

/// Write a single holding register in a remote device.
#[must_use]
pub struct WriteSingleRegister;

impl protocol::Function for WriteSingleRegister {
    const CODE: Code = Code::WriteSingleRegister;
    type Args = write_single_register::Payload;
    type Output = write_single_register::Payload;
}

/// Read the contents of eight Exception Status outputs in a remote device.
#[must_use]
#[derive(Copy, Clone)]
pub struct ReadExceptionStatus;

impl protocol::Function for ReadExceptionStatus {
    const CODE: Code = Code::ReadExceptionStatus;
    type Args = read_exception_status::Args;
    type Output = read_exception_status::Output;
}

/// Force each coil in a sequence of coils to either «on» or «off» in a remote device.
#[must_use]
pub struct WriteMultipleCoils;

impl protocol::Function for WriteMultipleCoils {
    const CODE: Code = Code::WriteMultipleCoils;
    type Args = write_multiple_coils::Args;
    type Output = write_multiple_coils::Output;
}

/// Write a block of contiguous registers (1 to 123 registers) in a remote device.
#[must_use]
pub struct WriteMultipleRegisters;

impl protocol::Function for WriteMultipleRegisters {
    const CODE: Code = Code::WriteMultipleRegisters;
    type Args = write_multiple_registers::Args;
    type Output = write_multiple_registers::Output;
}
