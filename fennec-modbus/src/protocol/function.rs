use core::marker::PhantomData;

use crate::protocol::{
    Address,
    Function,
    codec::{Decoder, Encoder},
    function::read::{Coils, DiscreteInputs, HoldingRegisters, InputRegisters},
};

pub mod read;

/// Associates function code with function type.
pub trait Code {
    /// Modbus function code.
    const CODE: u8;
}

/// Read function.
///
/// This type is an umbrella for the common read operations:
/// coils, discrete inputs, holding registers, and input registers.
///
/// - [`Code`] encodes the function code.
/// - Concrete [`Address`] implementation encodes address.
/// - [`read::ArgsEncoder`] encodes the address and the "quantity" parameter.
/// - Output type defines the number of coils or registers to read.
/// - Output decoder is responsible for decoding the output.
pub struct Read<C, A, V, D>(
    /// Binding to the function code.
    PhantomData<C>,
    /// Binding to the address type.
    PhantomData<A>,
    /// Binding to the output type.
    PhantomData<V>,
    /// Binding to the output decoder type.
    PhantomData<D>,
);

impl<A, V, D> Code for Read<Coils, A, V, D> {
    const CODE: u8 = 1;
}

impl<A, V, D> Code for Read<DiscreteInputs, A, V, D> {
    const CODE: u8 = 2;
}

impl<A, V, D> Code for Read<HoldingRegisters, A, V, D> {
    const CODE: u8 = 3;
}

impl<A, V, D> Code for Read<InputRegisters, A, V, D> {
    const CODE: u8 = 4;
}

impl<C, A, V, D> Function for Read<C, A, V, D>
where
    // Require that the function code is assigned:
    Self: Code,
    // Require address definition:
    A: Address,
    // Require arguments encoder implementation:
    read::ArgsEncoder<C, A, V>: Encoder<A::Args>,
    // Require that the output value decoder is implemented:
    D: Decoder<V>,
{
    type Args = A::Args;
    type ArgsEncoder = read::ArgsEncoder<C, A, V>;
    type Output = V;
    type OutputDecoder = read::OutputDecoder<V, D>;
}
