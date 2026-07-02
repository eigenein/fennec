//! Codes for functions that read multiple coils or registers.

use core::marker::PhantomData;

use bytes::{Buf, BufMut};

use crate::{
    Error,
    protocol::{
        Address,
        codec::{BitSize, Decode, Encode, adapters::DropRemaining},
        function,
        function::size_argument::SizeArgument,
    },
};

/// Address range for reading operations.
///
/// # Example
///
/// ```rust
/// use fennec_modbus::protocol::{
///     codec::Encode,
///     function::{read_multiple::Args, size_argument},
/// };
///
/// // Read holding registers 108–110 (Modbus spec §6.3 example).
/// assert_eq!(
///     Args::<_, [u16; 3], size_argument::Words>::new(0x006B_u16).to_bytes(),
///     [
///         0x00, 0x6B, // starting address
///         0x00, 0x03, // quantity of registers
///     ]
/// );
/// ```
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

impl<A: Address, V: BitSize, S: SizeArgument> Encode for Args<A, V, S> {
    /// Encode the address and number of bits to read.
    fn encode_to(&self, buf: &mut impl BufMut) {
        S::assert_valid_size::<V, 250>();
        self.0.encode_to(buf);
        buf.put_u16(S::quantity_for::<V>());
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
/// let value = Output::<u32>::decode_from(&mut BYTES).unwrap().into_value();
/// assert_eq!(value, 0x022B0000);
/// ```
pub struct Output<V>(V);

impl<V: Decode> Decode for Output<V> {
    fn decode_from(buf: &mut impl Buf) -> Result<Self, Error> {
        let n_bytes = buf.try_get_u8()?;
        let mut from = DropRemaining(buf).take(usize::from(n_bytes));
        V::decode_from(&mut from).map(Self)
    }
}

impl<V> function::IntoValue for Output<V> {
    type Value = V;

    fn into_value(self) -> Self::Value {
        self.0
    }
}
