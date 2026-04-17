use core::marker::PhantomData;

use bytes::BufMut;

use crate::protocol::{
    Address,
    codec::{BitSize, Encoder, NativeEndian},
};

/// Address specified in runtime when calling a function.
pub struct Runtime;

impl Address for Runtime {
    type Args = u16;
    type ArgsEncoder = NativeEndian;
}

/// Address specified via a constant generic argument.
pub struct Const<const A: u16>;

impl<const A: u16> Address for Const<A> {
    type Args = ();
    type ArgsEncoder = Self;
}

impl<const A: u16> Encoder<()> for Const<A> {
    fn encode((): &(), to: &mut impl BufMut) {
        to.put_u16(A);
    }
}

/// Address computed as [`BASE`] + size-of-[`V`] × `index`.
pub struct Stride<const BASE: u16, V>(PhantomData<V>);

impl<const BASE: u16, V: BitSize> Address for Stride<BASE, V> {
    type Args = u16;
    type ArgsEncoder = Self;
}

impl<const BASE: u16, V: BitSize> Encoder<u16> for Stride<BASE, V> {
    fn encode(index: &u16, to: &mut impl BufMut) {
        to.put_u16(BASE + V::N_WORDS * index);
    }
}
