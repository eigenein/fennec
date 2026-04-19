use core::marker::PhantomData;

use bytes::BufMut;

use crate::protocol::{
    codec::{BitSize, Encode},
    function::read_multiple,
};

/// Address specified via a constant generic argument.
#[must_use]
pub struct Const<const A: u16>;

impl<const A: u16> Encode for Const<A> {
    fn encode(&self, to: &mut impl BufMut) {
        to.put_u16(A);
    }
}

impl<const A: u16, V: BitSize, S> From<Const<A>> for read_multiple::AddressRange<Const<A>, V, S> {
    /// Convert the constant address into [`read_multiple::AddressRange`].
    fn from(r#const: Const<A>) -> Self {
        Self::new(r#const)
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

impl<const BASE: u16, V> Stride<BASE, V> {
    pub const fn new(index: u16) -> Self {
        Self(index, PhantomData)
    }
}

impl<const BASE: u16, V: BitSize> Encode for Stride<BASE, V> {
    fn encode(&self, to: &mut impl BufMut) {
        to.put_u16(BASE + self.0 * V::N_WORDS);
    }
}

impl<const BASE: u16, V: BitSize, S> From<Stride<BASE, V>>
    for read_multiple::AddressRange<Stride<BASE, V>, V, S>
{
    fn from(stride: Stride<BASE, V>) -> Self {
        Self::new(stride)
    }
}
