use core::marker::PhantomData;

use bytes::BufMut;

use crate::protocol::{
    Address,
    codec::{BitSize, Encode},
};

/// Address specified via a constant generic argument.
#[must_use]
#[derive(Copy, Clone)]
pub struct Const<const A: u16>;

impl<const A: u16> Address for Const<A> {}

impl<const A: u16> Encode for Const<A> {
    fn encode_to(&self, buf: &mut impl BufMut) {
        buf.put_u16(A);
    }
}

/// Address computed as `BASE` + size-of-`V` × `index`.
///
/// `BASE` is a bare register address. `N_STRIDES` is the number of valid strides.
#[must_use]
#[derive(Copy, Clone)]
pub struct Stride<const BASE: u16, const N_STRIDES: u16, V>(
    /// Value index within the stride.
    pub u16,
    /// Binding to the value type.
    PhantomData<V>,
);

impl<const BASE: u16, const N_STRIDES: u16, V> Stride<BASE, N_STRIDES, V> {
    /// Create a stride address at the specified index.
    ///
    /// # Panics
    ///
    /// The index violates the maximum number of strides for this type.
    pub const fn new(index: u16) -> Self {
        assert!(index < N_STRIDES, "the stride index is out of the bounds");
        Self(index, PhantomData)
    }
}

impl<const BASE: u16, const N_STRIDES: u16, V: BitSize> Address for Stride<BASE, N_STRIDES, V> {}

impl<const BASE: u16, const N_STRIDES: u16, V: BitSize> Encode for Stride<BASE, N_STRIDES, V> {
    fn encode_to(&self, buf: &mut impl BufMut) {
        buf.put_u16(BASE + self.0 * V::N_WORDS);
    }
}
