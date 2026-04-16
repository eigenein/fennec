//! Implementations of Modbus function arguments and outputs.

use alloc::vec::Vec;
use core::{marker::PhantomData, num::TryFromIntError};

use thiserror::Error;

use crate::{protocol, protocol::Decode};

pub mod read_registers;
pub mod write_multiple_registers;
pub mod write_single_coil;
pub mod write_single_register;

/// Function argument error.
///
/// These errors may be returned from arguments constructor functions.
#[derive(Debug, Error)]
pub enum ArgumentError {
    #[error("invalid number of registers ({0})")]
    InvalidRegisterCount(usize),

    #[error("value does not fit the target type")]
    TryFromInt(#[from] TryFromIntError),
}

/// Associates function code with function type.
pub trait Code {
    /// Modbus function code.
    const CODE: u8;
}

/// Read the contents of a contiguous block of registers in a remote device.
#[must_use]
#[derive(Copy, Clone)]
pub struct ReadRegisters<C, O>(PhantomData<(C, O)>);

impl<C: Code, O> Code for ReadRegisters<C, O> {
    const CODE: u8 = C::CODE;
}

impl<C: Code, V: Decode> protocol::Function for ReadRegisters<C, Vec<V>> {
    type Args = read_registers::Args<V>;
    type Output = read_registers::Output<Vec<V>>;
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
    type Args = ();
    type Output = u8;
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
