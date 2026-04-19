use core::marker::PhantomData;

use bytes::BufMut;

use crate::protocol::{
    Address,
    codec::{BitSize, Encode},
};

/// Address specified via a constant generic argument.
#[must_use]
pub struct Const<const A: u16>;

impl<const A: u16> Address for Const<A> {}

impl<const A: u16> Encode for Const<A> {
    fn encode(&self, to: &mut impl BufMut) {
        to.put_u16(A);
    }
}

/// Address computed as `BASE` + size-of-`V` × `index`.
#[must_use]
pub struct Stride<const BASE: u16, V>(
    /// Value index within the stride.
    pub u16,
    /// Binding to the value type.
    PhantomData<V>,
);

impl<const BASE: u16, V> From<u16> for Stride<BASE, V> {
    fn from(index: u16) -> Self {
        Self(index, PhantomData)
    }
}

impl<const BASE: u16, V: BitSize> Address for Stride<BASE, V> {}

impl<const BASE: u16, V: BitSize> Encode for Stride<BASE, V> {
    fn encode(&self, to: &mut impl BufMut) {
        to.put_u16(BASE + self.0 * V::N_WORDS);
    }
}
