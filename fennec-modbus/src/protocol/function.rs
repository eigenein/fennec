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

impl<S: Readable> protocol::FunctionCode for ReadCoils<S> {
    const CODE: u8 = 1;
}

impl<S: Readable> protocol::Function for ReadCoils<S> {
    type Args = read_coils::Args;
    type Output = read_coils::Output<S>;
}

/// Read from 1 to 2000 contiguous status of discrete inputs in a remote device.
#[derive(Copy, Clone)]
#[must_use]
pub struct ReadDiscreteInputs<S>(PhantomData<S>);

impl<S: Readable> protocol::FunctionCode for ReadDiscreteInputs<S> {
    const CODE: u8 = 2;
}

impl<S: Readable> protocol::Function for ReadDiscreteInputs<S> {
    type Args = read_discrete_inputs::Args;
    type Output = read_discrete_inputs::Output<S>;
}

/// Read the contents of a contiguous block of holding registers in a remote device.
#[must_use]
#[derive(Copy, Clone)]
pub struct ReadHoldingRegisters<V>(PhantomData<V>);

impl<V: read_registers::Value> protocol::FunctionCode for ReadHoldingRegisters<V> {
    const CODE: u8 = 3;
}

impl<V: read_registers::Value> protocol::Function for ReadHoldingRegisters<V> {
    type Args = read_registers::Args<V>;
    type Output = read_registers::Output<V>;
}

/// Read the contents of a contiguous block of holding registers in a remote device.
///
/// This is the same function as [`ReadHoldingRegisters`] – but with the register count known at compile time.
#[must_use]
#[derive(Copy, Clone)]
pub struct ReadHoldingRegistersExact<const N: usize, V>(PhantomData<V>);

impl<const N: usize, V: read_registers::Value> protocol::FunctionCode
    for ReadHoldingRegistersExact<N, V>
{
    const CODE: u8 = 3;
}

impl<const N: usize, V: read_registers::Value> protocol::Function
    for ReadHoldingRegistersExact<N, V>
{
    type Args = read_registers::Args<V>;
    type Output = read_registers::OutputExact<N, V>;
}

/// Read from 1 to 125 contiguous input registers in a remote device.
#[must_use]
#[derive(Copy, Clone)]
pub struct ReadInputRegisters<V>(PhantomData<V>);

impl<V: read_registers::Value> protocol::FunctionCode for ReadInputRegisters<V> {
    const CODE: u8 = 4;
}

impl<V: read_registers::Value> protocol::Function for ReadInputRegisters<V> {
    type Args = read_registers::Args<V>;
    type Output = read_registers::Output<V>;
}

/// Read from 1 to 125 contiguous input registers in a remote device.
///
/// This is the same function as [`ReadInputRegisters`] – but with the register count known at compile time.
#[must_use]
#[derive(Copy, Clone)]
pub struct ReadInputRegistersExact<const N: usize, V>(PhantomData<V>);

impl<const N: usize, V: read_registers::Value> protocol::FunctionCode
    for ReadInputRegistersExact<N, V>
{
    const CODE: u8 = 4;
}

impl<const N: usize, V: read_registers::Value> protocol::Function
    for ReadInputRegistersExact<N, V>
{
    type Args = read_registers::Args<V>;
    type Output = read_registers::OutputExact<N, V>;
}

/// Write a single output to either «on» or «off» in a remote device.
#[must_use]
pub struct WriteSingleCoil;

impl protocol::FunctionCode for WriteSingleCoil {
    const CODE: u8 = 5;
}

impl protocol::Function for WriteSingleCoil {
    type Args = write_single_coil::Payload;
    type Output = write_single_coil::Payload;
}

/// Write a single holding register in a remote device.
#[must_use]
pub struct WriteSingleRegister;

impl protocol::FunctionCode for WriteSingleRegister {
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

impl protocol::FunctionCode for ReadExceptionStatus {
    const CODE: u8 = 7;
}

impl protocol::Function for ReadExceptionStatus {
    type Args = read_exception_status::Args;
    type Output = read_exception_status::Output;
}

/// Force each coil in a sequence of coils to either «on» or «off» in a remote device.
#[must_use]
pub struct WriteMultipleCoils;

impl protocol::FunctionCode for WriteMultipleCoils {
    const CODE: u8 = 15;
}

impl protocol::Function for WriteMultipleCoils {
    type Args = write_multiple_coils::Args;
    type Output = write_multiple_coils::Output;
}

/// Write a block of contiguous registers (1 to 123 registers) in a remote device.
#[must_use]
pub struct WriteMultipleRegisters;

impl protocol::FunctionCode for WriteMultipleRegisters {
    const CODE: u8 = 16;
}

impl protocol::Function for WriteMultipleRegisters {
    type Args = write_multiple_registers::Args;
    type Output = write_multiple_registers::Output;
}
