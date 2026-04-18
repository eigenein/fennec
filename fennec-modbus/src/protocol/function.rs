use core::marker::PhantomData;

use crate::protocol::{
    Address,
    Function,
    codec::{Decoder, Encoder},
};

pub mod read;
mod size;

/// Associates function code with function type.
pub trait Code {
    /// Modbus function code.
    const CODE: u8;
}

/// Read coils.
///
/// Type parameters bind to the address, value, and codec types.
pub struct ReadCoils<A, V, C>(PhantomData<(A, V, C)>);

impl<A, V, D> Code for ReadCoils<A, V, D> {
    const CODE: u8 = 1;
}

/// Read discrete inputs.
pub struct ReadDiscreteInputs<A, V, C>(PhantomData<(A, V, C)>);

impl<A, V, C> Code for ReadDiscreteInputs<A, V, C> {
    const CODE: u8 = 2;
}

/// Read holding registers.
pub struct ReadHoldingRegisters<A, V, C>(PhantomData<(A, V, C)>);

impl<A, V, C> Code for ReadHoldingRegisters<A, V, C> {
    const CODE: u8 = 3;
}

impl<A, V, D> Function for ReadHoldingRegisters<A, V, D>
where
    // Require that the function code is assigned:
    Self: Code,
    // Require address definition:
    A: Address,
    // Require arguments encoder implementation:
    read::ArgsEncoder<A, V, size::Words>: Encoder<A::Args>,
    // Require that the output value decoder is implemented:
    D: Decoder<V>,
{
    type Args = A::Args;
    type ArgsEncoder = read::ArgsEncoder<A, V, size::Words>;
    type Output = V;
    type OutputDecoder = read::OutputDecoder<V, D>;
}

/// Read input registers.
pub struct ReadInputRegisters<A, V, C>(PhantomData<(A, V, C)>);

impl<A, V, C> Code for ReadInputRegisters<A, V, C> {
    const CODE: u8 = 4;
}

/// Write multiple registers.
pub struct WriteMultipleRegisters<A, V, C>(PhantomData<(A, V, C)>);

impl<A, V, C> Code for WriteMultipleRegisters<A, V, C> {
    const CODE: u8 = 16;
}
