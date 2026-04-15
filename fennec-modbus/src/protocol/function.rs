//! Implementations of Modbus function arguments and outputs.

use core::marker::PhantomData;

use deku::DekuContainerRead;

use crate::protocol;

pub mod read_coils;
pub mod read_discrete_inputs;
pub mod read_exception_status;
pub mod read_registers;
pub mod write_multiple_coils;
pub mod write_multiple_registers;
pub mod write_single_coil;
pub mod write_single_register;

/// Associates function code with function type.
pub trait Code {
    /// Modbus function code.
    const CODE: u8;
}

/// Read from 1 to 2000 contiguous status of coils in a remote device.
#[derive(Copy, Clone)]
#[must_use]
pub struct ReadCoils<S>(PhantomData<S>);

impl<S> Code for ReadCoils<S> {
    const CODE: u8 = 1;
}

impl<S: for<'a> DekuContainerRead<'a>> protocol::Function for ReadCoils<S> {
    type Args = read_coils::Args;
    type Output = read_coils::Output<S>;
}

/// Read from 1 to 2000 contiguous status of discrete inputs in a remote device.
#[derive(Copy, Clone)]
#[must_use]
pub struct ReadDiscreteInputs<S>(PhantomData<S>);

impl<S> Code for ReadDiscreteInputs<S> {
    const CODE: u8 = 2;
}

impl<S: for<'a> DekuContainerRead<'a>> protocol::Function for ReadDiscreteInputs<S> {
    type Args = read_discrete_inputs::Args;
    type Output = read_discrete_inputs::Output<S>;
}

/// Read the contents of a contiguous block of registers in a remote device.
#[must_use]
#[derive(Copy, Clone)]
pub struct ReadRegisters<C, V>(PhantomData<(C, V)>);

impl<C: Code, V> Code for ReadRegisters<C, V> {
    const CODE: u8 = C::CODE;
}

impl<C: Code, V: for<'a> DekuContainerRead<'a>> protocol::Function for ReadRegisters<C, V> {
    type Args = read_registers::Args<V>;
    type Output = read_registers::Output<V>;
}

/// Write a single output to either «on» or «off» in a remote device.
#[must_use]
pub struct WriteSingleCoil;

impl Code for WriteSingleCoil {
    const CODE: u8 = 5;
}

impl protocol::Function for WriteSingleCoil {
    type Args = write_single_coil::Payload;
    type Output = write_single_coil::Payload;
}

/// Write a single holding register in a remote device.
#[must_use]
pub struct WriteSingleRegister;

impl Code for WriteSingleRegister {
    const CODE: u8 = 6;
}

impl protocol::Function for WriteSingleRegister {
    type Args = write_single_register::Payload;
    type Output = write_single_register::Payload;
}

/// Read the contents of eight Exception Status outputs in a remote device.
#[must_use]
#[derive(Copy, Clone)]
pub struct ReadExceptionStatus;

impl Code for ReadExceptionStatus {
    const CODE: u8 = 7;
}

impl protocol::Function for ReadExceptionStatus {
    type Args = read_exception_status::Args;
    type Output = read_exception_status::Output;
}

/// Force each coil in a sequence of coils to either «on» or «off» in a remote device.
#[must_use]
pub struct WriteMultipleCoils;

impl Code for WriteMultipleCoils {
    const CODE: u8 = 15;
}

impl protocol::Function for WriteMultipleCoils {
    type Args = write_multiple_coils::Args;
    type Output = write_multiple_coils::Output;
}

/// Write a block of contiguous registers (1 to 123 registers) in a remote device.
#[must_use]
pub struct WriteMultipleRegisters;

impl Code for WriteMultipleRegisters {
    const CODE: u8 = 16;
}

impl protocol::Function for WriteMultipleRegisters {
    type Args = write_multiple_registers::Args;
    type Output = write_multiple_registers::Output;
}
