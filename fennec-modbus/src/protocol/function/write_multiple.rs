//! Codecs for functions that write multiple coils or registers.

use core::marker::PhantomData;

use bytes::{Buf, BufMut};

use crate::{
    Error,
    protocol::{
        Address,
        codec::{BitSize, Decode, Encode},
        function::{IntoValue, size_argument},
    },
};

/// Address range and values for writing operations.
///
/// # Example
///
/// ```rust
/// use fennec_modbus::protocol::{codec::Encode, function::write_multiple::Args};
///
/// assert_eq!(
///     Args::new(1_u16, [0x000A_u16, 0x0102]).to_bytes(),
///     [
///         0x00, 0x01, // starting address
///         0x00, 0x02, // register count
///         0x04, // byte count
///         0x00, 0x0A, // register 1
///         0x01, 0x02, // register 2
///     ]
/// );
/// ```
pub struct Args<A, V, S>(
    /// Bare starting address.
    A,
    /// Value to write.
    V,
    /// Binding to the size type, normally [`size_argument::Bits`] or [`size_argument::Words`].
    PhantomData<S>,
);

impl<A, V, S> Args<A, V, S> {
    pub const fn new(address: A, value: V) -> Self {
        Self(address, value, PhantomData)
    }
}

impl<A: Address, V: BitSize + Encode> Encode for Args<A, V, size_argument::Words> {
    fn encode(&self, to: &mut impl BufMut) {
        V::assert_valid::<246>();
        self.0.encode(to);
        to.put_u16(V::N_WORDS);
        to.put_u8(V::N_BYTES);
        self.1.encode(to);
    }
}

/// Writing output.
///
/// # Example
///
/// ```rust
/// use fennec_modbus::protocol::{codec::Decode, function::write_multiple::Output};
///
/// let mut bytes: &[u8] = &[0x00, 0x01, 0x00, 0x02];
/// let output = Output::decode(&mut bytes).unwrap();
/// assert_eq!(output.starting_address, 1);
/// assert_eq!(output.count, 2);
/// ```
pub struct Output {
    /// Starting address.
    pub starting_address: u16,

    /// Count of registers or coils – according to the operation.
    pub count: u16,
}

impl IntoValue for Output {
    type Value = Self;

    fn into_value(self) -> Self::Value {
        self
    }
}

impl Decode for Output {
    fn decode(from: &mut impl Buf) -> Result<Self, Error> {
        Ok(Self { starting_address: u16::decode(from)?, count: u16::decode(from)? })
    }
}
