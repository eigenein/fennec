//! Implementations of Modbus function arguments and outputs.

use core::marker::PhantomData;

use crate::{protocol, protocol::r#struct::Readable};

pub mod read_coils;
pub mod read_discrete_inputs;
pub mod read_exception_status;
pub mod read_registers;
pub mod write_multiple_coils;
pub mod write_multiple_registers;
pub mod write_single_coil;
pub mod write_single_register;

/// Read from 1 to 2000 contiguous status of coils in a remote device.
#[derive(Copy, Clone)]
#[must_use]
pub struct ReadCoils<S: Readable>(PhantomData<S>);

impl<S: Readable> protocol::Function for ReadCoils<S> {
    const CODE: u8 = 1;
    type Args = read_coils::Args;
    type Output = read_coils::Output<S>;
}

/// Read from 1 to 2000 contiguous status of discrete inputs in a remote device.
#[derive(Copy, Clone)]
#[must_use]
pub struct ReadDiscreteInputs<S>(PhantomData<S>);

impl<S: Readable> protocol::Function for ReadDiscreteInputs<S> {
    const CODE: u8 = 2;
    type Args = read_discrete_inputs::Args;
    type Output = read_discrete_inputs::Output<S>;
}

/// Read the contents of eight Exception Status outputs in a remote device.
#[must_use]
#[derive(Copy, Clone)]
pub struct ReadExceptionStatus;

impl protocol::Function for ReadExceptionStatus {
    const CODE: u8 = 7;
    type Args = read_exception_status::Args;
    type Output = read_exception_status::Output;
}

/// Read the contents of a contiguous block of holding registers in a remote device.
#[must_use]
pub struct ReadHoldingRegisters;

impl protocol::Function for ReadHoldingRegisters {
    const CODE: u8 = 3;
    type Args = read_registers::Args;
    type Output = read_registers::Output;
}

/// Read from 1 to 125 contiguous input registers in a remote device.
#[must_use]
pub struct ReadInputRegisters;

impl protocol::Function for ReadInputRegisters {
    const CODE: u8 = 4;
    type Args = read_registers::Args;
    type Output = read_registers::Output;
}

/// Force each coil in a sequence of coils to either «on» or «off» in a remote device.
#[must_use]
pub struct WriteMultipleCoils;

impl protocol::Function for WriteMultipleCoils {
    const CODE: u8 = 15;
    type Args = write_multiple_coils::Args;
    type Output = write_multiple_coils::Output;
}

/// Write a block of contiguous registers (1 to 123 registers) in a remote device.
#[must_use]
pub struct WriteMultipleRegisters;

impl protocol::Function for WriteMultipleRegisters {
    const CODE: u8 = 16;
    type Args = write_multiple_registers::Args;
    type Output = write_multiple_registers::Output;
}

/// Write a single output to either «on» or «off» in a remote device.
#[must_use]
pub struct WriteSingleCoil;

impl protocol::Function for WriteSingleCoil {
    const CODE: u8 = 5;
    type Args = write_single_coil::Payload;
    type Output = write_single_coil::Payload;
}

/// Write a single holding register in a remote device.
#[must_use]
pub struct WriteSingleRegister;

impl protocol::Function for WriteSingleRegister {
    const CODE: u8 = 6;
    type Args = write_single_register::Payload;
    type Output = write_single_register::Payload;
}
