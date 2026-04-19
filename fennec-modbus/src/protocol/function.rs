use core::marker::PhantomData;

use crate::protocol::{
    Address,
    Function,
    codec::{Decoder, Encoder},
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
pub struct ReadCoils<A, V, C>(PhantomData<(A, V, C)>);

impl<A, V, C> Code for ReadCoils<A, V, C> {
    const CODE: u8 = 1;
}

impl<A, V, C> Function for ReadCoils<A, V, C>
where
    A: Address,
    read_multiple::ArgsEncoder<A, V, size_argument::Bits>: Encoder<A::Args>,
    C: Decoder<V>,
{
    type Args = A::Args;
    type ArgsEncoder = read_multiple::ArgsEncoder<A, V, size_argument::Bits>;
    type Output = V;
    type OutputDecoder = read_multiple::OutputDecoder<V, C>;
}

/// Read discrete inputs.
pub struct ReadDiscreteInputs<A, V, C>(PhantomData<(A, V, C)>);

impl<A, V, C> Code for ReadDiscreteInputs<A, V, C> {
    const CODE: u8 = 2;
}

impl<A, V, C> Function for ReadDiscreteInputs<A, V, C>
where
    A: Address,
    read_multiple::ArgsEncoder<A, V, size_argument::Bits>: Encoder<A::Args>,
    C: Decoder<V>,
{
    type Args = A::Args;
    type ArgsEncoder = read_multiple::ArgsEncoder<A, V, size_argument::Bits>;
    type Output = V;
    type OutputDecoder = read_multiple::OutputDecoder<V, C>;
}

/// Read holding registers.
pub struct ReadHoldingRegisters<A, V, C>(PhantomData<(A, V, C)>);

impl<A, V, C> Code for ReadHoldingRegisters<A, V, C> {
    const CODE: u8 = 3;
}

impl<A, V, C> Function for ReadHoldingRegisters<A, V, C>
where
    A: Address,
    read_multiple::ArgsEncoder<A, V, size_argument::Words>: Encoder<A::Args>,
    C: Decoder<V>,
{
    type Args = A::Args;
    type ArgsEncoder = read_multiple::ArgsEncoder<A, V, size_argument::Words>;
    type Output = V;
    type OutputDecoder = read_multiple::OutputDecoder<V, C>;
}

/// Read input registers.
pub struct ReadInputRegisters<A, V, C>(PhantomData<(A, V, C)>);

impl<A, V, C> Code for ReadInputRegisters<A, V, C> {
    const CODE: u8 = 4;
}

impl<A, V, C> Function for ReadInputRegisters<A, V, C>
where
    A: Address,
    read_multiple::ArgsEncoder<A, V, size_argument::Words>: Encoder<A::Args>,
    C: Decoder<V>,
{
    type Args = A::Args;
    type ArgsEncoder = read_multiple::ArgsEncoder<A, V, size_argument::Words>;
    type Output = V;
    type OutputDecoder = read_multiple::OutputDecoder<V, C>;
}

/// Write multiple registers.
pub struct WriteMultipleRegisters<A, V, C>(PhantomData<(A, V, C)>);

impl<A, V, C> Code for WriteMultipleRegisters<A, V, C> {
    const CODE: u8 = 16;
}

/*
impl<A, V, C> Function for WriteMultipleRegisters<A, V, C>
where
    A: Address,
    write_multiple::ArgsEncoder<A, V, size_argument::Words>: Encoder<A::Args>,
    C: Encoder<V>,
{
    type Args = todo!();
    type ArgsEncoder = write_multiple::ArgsEncoder<A, V, size_argument::Words>;
    type Output = todo!();
    type OutputDecoder = todo!();
}
*/
