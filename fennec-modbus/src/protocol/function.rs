use core::marker::PhantomData;

use crate::protocol::{
    Function,
    codec::{Decode, Encode},
};

pub mod read_multiple;
pub mod read_write_multiple;
pub mod size_argument;
pub mod write_multiple;

/// Associates function code with function type.
pub trait Code {
    /// Modbus function code.
    const CODE: u8;
}

/// Function response output.
pub trait IntoValue {
    type Value;

    /// Unwrap the function response output.
    fn into_value(self) -> Self::Value;
}

/// Read coils.
///
/// Type parameters bind to the address, value, and codec types.
#[must_use]
pub struct ReadCoils<A, V>(PhantomData<(A, V)>);

impl<A, V> Code for ReadCoils<A, V> {
    const CODE: u8 = 1;
}

impl<A, V> Function for ReadCoils<A, V>
where
    read_multiple::Args<A, V, size_argument::Bits>: Encode,
    read_multiple::Output<V>: Decode,
{
    type Args = read_multiple::Args<A, V, size_argument::Bits>;
    type Output = read_multiple::Output<V>;
}

/// Read discrete inputs.
#[must_use]
pub struct ReadDiscreteInputs<A, V>(PhantomData<(A, V)>);

impl<A, V> Code for ReadDiscreteInputs<A, V> {
    const CODE: u8 = 2;
}

impl<A, V> Function for ReadDiscreteInputs<A, V>
where
    read_multiple::Args<A, V, size_argument::Bits>: Encode,
    read_multiple::Output<V>: Decode,
{
    type Args = read_multiple::Args<A, V, size_argument::Bits>;
    type Output = read_multiple::Output<V>;
}

/// Read holding registers.
#[must_use]
pub struct ReadHoldingRegisters<A, V>(PhantomData<(A, V)>);

impl<A, V> Code for ReadHoldingRegisters<A, V> {
    const CODE: u8 = 3;
}

impl<A, V> Function for ReadHoldingRegisters<A, V>
where
    read_multiple::Args<A, V, size_argument::Words>: Encode,
    read_multiple::Output<V>: Decode,
{
    type Args = read_multiple::Args<A, V, size_argument::Words>;
    type Output = read_multiple::Output<V>;
}

/// Read input registers.
#[must_use]
pub struct ReadInputRegisters<A, V>(PhantomData<(A, V)>);

impl<A, V> Code for ReadInputRegisters<A, V> {
    const CODE: u8 = 4;
}

impl<A, V> Function for ReadInputRegisters<A, V>
where
    read_multiple::Args<A, V, size_argument::Words>: Encode,
    read_multiple::Output<V>: Decode,
{
    type Args = read_multiple::Args<A, V, size_argument::Words>;
    type Output = read_multiple::Output<V>;
}

/// Write multiple registers.
#[must_use]
pub struct WriteMultipleRegisters<A, V>(PhantomData<(A, V)>);

impl<A, V> Code for WriteMultipleRegisters<A, V> {
    const CODE: u8 = 16;
}

impl<A, V> Function for WriteMultipleRegisters<A, V>
where
    write_multiple::Args<A, V, size_argument::Words>: Encode,
{
    type Args = write_multiple::Args<A, V, size_argument::Words>;
    type Output = write_multiple::Output;
}

/// Read and write multiple registers.
///
/// The write operation is performed *before* the read.
#[must_use]
pub struct ReadWriteRegisters<ReadAddress, ReadValue, WriteAddress, WriteValue>(
    /// Binding to the address type to read.
    PhantomData<ReadAddress>,
    /// Binding to the value type to read.
    PhantomData<ReadValue>,
    /// Binding to the address type to write.
    PhantomData<WriteAddress>,
    /// Binding to the value type to write.
    PhantomData<WriteValue>,
);

impl<ReadAddress, ReadValue, WriteAddress, WriteValue> Code
    for ReadWriteRegisters<ReadAddress, ReadValue, WriteAddress, WriteValue>
{
    const CODE: u8 = 23;
}

impl<ReadAddress, ReadValue, WriteAddress, WriteValue> Function
    for ReadWriteRegisters<ReadAddress, ReadValue, WriteAddress, WriteValue>
where
    read_write_multiple::Args<ReadAddress, ReadValue, WriteAddress, WriteValue>: Encode,
    read_multiple::Output<ReadValue>: Decode,
{
    type Args = read_write_multiple::Args<ReadAddress, ReadValue, WriteAddress, WriteValue>;
    type Output = read_multiple::Output<ReadValue>;
}
