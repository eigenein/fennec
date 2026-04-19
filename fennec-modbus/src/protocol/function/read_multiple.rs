//! Codes for functions that read multiple coils or registers.

use core::marker::PhantomData;

use bytes::{Buf, BufMut};

use crate::{
    Error,
    protocol::{
        Address,
        codec::{BitSize, Decode, Encode, adapters::DropRemaining},
        function,
        function::size_argument,
    },
};

/// Address range for reading operations.
#[must_use]
pub struct Args<A, V, S>(
    /// Bare starting address.
    A,
    /// Binding to the value type. This is needed to know the number of registers or coils.
    PhantomData<V>,
    /// Binding to the size type, normally [`size_argument::Bits`] or [`size_argument::Words`].
    PhantomData<S>,
);

impl<A, V: BitSize, S> From<A> for Args<A, V, S> {
    /// Wrap the address into [`Args`].
    fn from(address: A) -> Self {
        Self::new(address)
    }
}

impl<A, V: BitSize, S> Args<A, V, S> {
    /// Create the address range from the starting address.
    pub const fn new(starting_address: A) -> Self {
        Self(starting_address, PhantomData, PhantomData)
    }
}

impl<A: Address, V: BitSize> Encode for Args<A, V, size_argument::Bits> {
    /// Encode the address and number of bits to read.
    fn encode(&self, to: &mut impl BufMut) {
        V::assert_valid::<246>();
        self.0.encode(to);
        to.put_u16(V::N_BITS);
    }
}

impl<A: Address, V: BitSize> Encode for Args<A, V, size_argument::Words> {
    /// Encode the address and number of registers to read.
    fn encode(&self, to: &mut impl BufMut) {
        V::assert_valid::<250>();
        self.0.encode(to);
        to.put_u16(V::N_WORDS);
    }
}

/// Output decoder for the read operations.
///
/// # Example
///
/// ```rust
/// use fennec_modbus::protocol::{
///     codec::Decode,
///     function::{IntoValue, read_multiple::Output},
/// };
///
/// const BYTES: &[u8] = &[
///     0x04, // byte count
///     0x02, 0x2B, // register: high, low
///     0x00, 0x00, // register: high, low
/// ];
///
/// #[expect(const_item_mutation)]
/// let value = Output::<u32>::decode(&mut BYTES).unwrap().into_value();
/// assert_eq!(value, 0x022B0000);
/// ```
pub struct Output<V>(V);

impl<V: Decode> Decode for Output<V> {
    fn decode(from: &mut impl Buf) -> Result<Self, Error> {
        let n_bytes = from.try_get_u8()?;
        let mut from = DropRemaining(from).take(usize::from(n_bytes));
        V::decode(&mut from).map(Self)
    }
}

impl<V> function::IntoValue for Output<V> {
    type Value = V;

    fn into_value(self) -> Self::Value {
        self.0
    }
}
