use core::marker::PhantomData;

use crate::protocol::{
    Function,
    codec::{Decode, Encode},
};

pub mod read_multiple;
mod size_argument;
pub mod write_multiple;

/// Associates function code with function type.
pub trait Code {
    /// Modbus function code.
    const CODE: u8;
}

/// Read coils.
///
/// Type parameters bind to the address, value, and codec types.
pub struct ReadCoils<A, V>(PhantomData<(A, V)>);

impl<A, V> Code for ReadCoils<A, V> {
    const CODE: u8 = 1;
}

impl<A, V> Function for ReadCoils<A, V>
where
    read_multiple::AddressRange<A, V, size_argument::Bits>: Encode,
    V: Decode,
{
    type Args = read_multiple::AddressRange<A, V, size_argument::Bits>;
    type Output = read_multiple::Output<V>;
}

/// Read discrete inputs.
pub struct ReadDiscreteInputs<A, V>(PhantomData<(A, V)>);

impl<A, V> Code for ReadDiscreteInputs<A, V> {
    const CODE: u8 = 2;
}

impl<A, V> Function for ReadDiscreteInputs<A, V>
where
    read_multiple::AddressRange<A, V, size_argument::Bits>: Encode,
    V: Decode,
{
    type Args = read_multiple::AddressRange<A, V, size_argument::Bits>;
    type Output = read_multiple::Output<V>;
}

/// Read holding registers.
pub struct ReadHoldingRegisters<A, V>(PhantomData<(A, V)>);

impl<A, V> Code for ReadHoldingRegisters<A, V> {
    const CODE: u8 = 3;
}

impl<A, V> Function for ReadHoldingRegisters<A, V>
where
    read_multiple::AddressRange<A, V, size_argument::Words>: Encode,
    V: Decode,
{
    type Args = read_multiple::AddressRange<A, V, size_argument::Words>;
    type Output = read_multiple::Output<V>;
}

/// Read input registers.
pub struct ReadInputRegisters<A, V>(PhantomData<(A, V)>);

impl<A, V> Code for ReadInputRegisters<A, V> {
    const CODE: u8 = 4;
}

impl<A, V> Function for ReadInputRegisters<A, V>
where
    read_multiple::AddressRange<A, V, size_argument::Words>: Encode,
    V: Decode,
{
    type Args = read_multiple::AddressRange<A, V, size_argument::Words>;
    type Output = read_multiple::Output<V>;
}

/// Write multiple registers.
pub struct WriteMultipleRegisters<A, V, C>(PhantomData<(A, V, C)>);

impl<A, V, C> Code for WriteMultipleRegisters<A, V, C> {
    const CODE: u8 = 16;
}
