use core::marker::PhantomData;

use crate::protocol::function::read::{Coils, DiscreteInputs, HoldingRegisters, InputRegisters};

pub mod read;

/// Associates function code with function type.
pub trait Code {
    /// Modbus function code.
    const CODE: u8;
}

pub struct Read<C, V>(PhantomData<(C, V)>);

impl<V> Code for Read<Coils, V> {
    const CODE: u8 = 1;
}

impl<V> Code for Read<DiscreteInputs, V> {
    const CODE: u8 = 2;
}

impl<V> Code for Read<HoldingRegisters, V> {
    const CODE: u8 = 3;
}

impl<V> Code for Read<InputRegisters, V> {
    const CODE: u8 = 4;
}
