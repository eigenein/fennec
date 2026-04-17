use bytes::BufMut;

use crate::protocol::{
    Address,
    codec::{Encoder, NativeEndian},
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
