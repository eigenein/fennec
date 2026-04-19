//! Codes for functions that read multiple coils or registers.

use core::marker::PhantomData;

use bytes::{Buf, BufMut};

use crate::{
    Error,
    protocol::{
        Address,
        codec::{BitSize, Decoder, Encoder, adapters::DropRemaining},
        function::size_argument,
    },
};

pub struct ArgsEncoder<A, V, S>(
    /// Binding to the address type.
    PhantomData<A>,
    /// Binding to the value type.
    PhantomData<V>,
    /// Binding to the size type, normally [`size_argument::Bits`] or [`size_argument::Words`].
    PhantomData<S>,
);

impl<A, V: BitSize, S> ArgsEncoder<A, V, S> {
    const fn assert_valid<const N_MAX_BYTES: u16>() {
        const {
            assert!(V::N_BYTES >= 1, "value type must be non-empty");
            assert!(V::N_BYTES <= N_MAX_BYTES, "value is too large");
        };
    }
}

impl<A: Address, V: BitSize> Encoder<A::Args> for ArgsEncoder<A, V, size_argument::Bits> {
    /// Encode the address and number of bits to read.
    fn encode(starting_address: &A::Args, to: &mut impl BufMut) {
        Self::assert_valid::<246>();
        A::ArgsEncoder::encode(starting_address, to);
        to.put_u16(V::N_BITS);
    }
}

impl<A: Address, V: BitSize> Encoder<A::Args> for ArgsEncoder<A, V, size_argument::Words> {
    /// Encode the address and number of registers to read.
    fn encode(starting_address: &A::Args, to: &mut impl BufMut) {
        Self::assert_valid::<250>();
        A::ArgsEncoder::encode(starting_address, to);
        to.put_u16(V::N_WORDS);
    }
}

/// Output decoder for the read operations.
///
/// # Example
///
/// ```rust
/// use fennec_modbus::protocol::{
///     codec::{BigEndian, Decoder},
///     function::read_multiple::OutputDecoder,
/// };
///
/// const BYTES: &[u8] = &[
///     0x04, // byte count
///     0x02, 0x2B, // register: high, low
///     0x00, 0x00, // register: high, low
/// ];
///
/// let value = OutputDecoder::<u32, BigEndian>::decode(&mut BYTES).unwrap();
/// assert_eq!(value, 0x022B0000);
/// ```
pub struct OutputDecoder<V, C>(
    /// Binding to the value type.
    PhantomData<V>,
    /// Binding to the value decoder type.
    PhantomData<C>,
);

impl<V, C: Decoder<V>> Decoder<V> for OutputDecoder<V, C> {
    fn decode(from: &mut impl Buf) -> Result<V, Error> {
        let n_bytes = from.try_get_u8()?;
        let mut from = DropRemaining(from).take(usize::from(n_bytes));
        C::decode(&mut from)
    }
}
