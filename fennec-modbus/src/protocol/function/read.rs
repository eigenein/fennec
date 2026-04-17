use core::marker::PhantomData;

use bytes::BufMut;

use crate::protocol::codec::{BitSize, Encoder};

/// Read coils.
pub struct Coils;

/// Read discrete inputs.
pub struct DiscreteInputs;

/// Read holding registers.
pub struct HoldingRegisters;

/// Read input registers.
pub struct InputRegisters;

/// Encodes starting address passed via the function arguments,
/// and the number of registers inferred from the target value size.
pub struct ArgsEncoder<C, V>(
    /// Binding to the function.
    PhantomData<C>,
    /// Binding to the output type.
    PhantomData<V>,
);

impl<C, V: BitSize> ArgsEncoder<C, V> {
    const fn assert_valid() {
        const {
            assert!(V::N_BYTES >= 1, "value type must be non-empty");
            assert!(V::N_BYTES <= 250, "value may be at most 250 bytes large");
        };
    }
}

impl<V: BitSize> Encoder<u16> for ArgsEncoder<HoldingRegisters, V> {
    /// Encode the address and number of holding registers to read.
    fn encode(starting_address: &u16, to: &mut impl BufMut) {
        Self::assert_valid();
        to.put_u16(*starting_address);
        to.put_u16(V::N_WORDS);
    }
}

impl<V: BitSize> Encoder<u16> for ArgsEncoder<InputRegisters, V> {
    /// Encode the address and number of input registers to read.
    fn encode(starting_address: &u16, to: &mut impl BufMut) {
        Self::assert_valid();
        to.put_u16(*starting_address);
        to.put_u16(V::N_WORDS);
    }
}
