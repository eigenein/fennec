use core::marker::PhantomData;

use bytes::{Buf, BufMut};

use crate::{
    Error,
    protocol::{
        Address,
        codec::{BitSize, Decoder, Encoder, adapters::DropRemaining},
    },
};

/// Read coils.
///
/// TODO: implement argument encoder.
pub struct Coils;

/// Read discrete inputs.
///
/// TODO: implement argument encoder.
pub struct DiscreteInputs;

/// Read holding registers.
pub struct HoldingRegisters;

/// Read input registers.
pub struct InputRegisters;

/// Encodes:
/// - starting address passed via the function arguments;
/// - the number of coils or registers (inferred from the target value size).
pub struct ArgsEncoder<C, A, V>(
    /// Binding to the function.
    PhantomData<C>,
    /// Binding to the address type.
    PhantomData<A>,
    /// Binding to the output type.
    PhantomData<V>,
);

impl<C, A, V: BitSize> ArgsEncoder<C, A, V> {
    const fn assert_valid() {
        const {
            assert!(V::N_BYTES >= 1, "value type must be non-empty");
            assert!(V::N_BYTES <= 250, "value may be at most 250 bytes large");
        };
    }
}

impl<A: Address, V: BitSize> Encoder<A::Args> for ArgsEncoder<HoldingRegisters, A, V> {
    /// Encode the address and number of holding registers to read.
    fn encode(starting_address: &A::Args, to: &mut impl BufMut) {
        Self::assert_valid();
        A::ArgsEncoder::encode(starting_address, to);
        to.put_u16(V::N_WORDS);
    }
}

impl<A: Address, V: BitSize> Encoder<A::Args> for ArgsEncoder<InputRegisters, A, V> {
    /// Encode the address and number of input registers to read.
    fn encode(starting_address: &A::Args, to: &mut impl BufMut) {
        Self::assert_valid();
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
///     function::read::OutputDecoder,
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
pub struct OutputDecoder<V, D>(
    /// Binding to the value type.
    PhantomData<V>,
    /// Binding to the value decoder type.
    PhantomData<D>,
);

impl<V, D: Decoder<V>> Decoder<V> for OutputDecoder<V, D> {
    fn decode(from: &mut impl Buf) -> Result<V, Error> {
        let n_bytes = from.try_get_u8()?;
        let mut from = DropRemaining(from).take(usize::from(n_bytes));
        D::decode(&mut from)
    }
}
