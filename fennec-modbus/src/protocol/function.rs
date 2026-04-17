use core::marker::PhantomData;

use crate::protocol::{
    Function,
    codec::{Decoder, Encoder},
    function::{
        address::AddressAndCountEncoder,
        read::{Coils, DiscreteInputs, HoldingRegisters, InputRegisters},
    },
};

pub mod address;
pub mod read;

/// Associates function code with function type.
pub trait Code {
    /// Modbus function code.
    const CODE: u8;
}

pub struct Read<C, V, D>(
    /// Binding to the function code.
    PhantomData<C>,
    /// Binding to the output type.
    PhantomData<V>,
    /// Binding to the output decoder type.
    PhantomData<D>,
);

impl<V, D> Code for Read<Coils, V, D> {
    const CODE: u8 = 1;
}

impl<V, D> Code for Read<DiscreteInputs, V, D> {
    const CODE: u8 = 2;
}

impl<V, D> Code for Read<HoldingRegisters, V, D> {
    const CODE: u8 = 3;
}

impl<C, V, D> Function for Read<C, V, D>
where
    // Require that the function code is assigned:
    Self: Code,
    // Require that the argument encoder is implemented:
    AddressAndCountEncoder<C, V>: Encoder<u16>,
    // Require that the output value decoder is implemented:
    D: Decoder<V>,
{
    /// Starting address.
    type Args = u16;

    type ArgsEncoder = AddressAndCountEncoder<C, V>;

    type Output = V;

    type OutputDecoder = D;
}

impl<V, D> Code for Read<InputRegisters, V, D> {
    const CODE: u8 = 4;
}
