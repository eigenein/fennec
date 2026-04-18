use bytes::BufMut;

use crate::protocol::codec::Word;

pub trait Encoder<T: ?Sized> {
    fn encode(value: &T, to: &mut impl BufMut);
}

impl Encoder<u16> for Word {
    fn encode(value: &u16, to: &mut impl BufMut) {
        to.put_u16(*value);
    }
}

impl Encoder<i16> for Word {
    fn encode(value: &i16, to: &mut impl BufMut) {
        to.put_i16(*value);
    }
}

macro_rules! impl_be {
    ($type:ty => $encode:ident) => {
        impl Encoder<$type> for crate::protocol::codec::BigEndian {
            fn encode(value: &$type, into: &mut impl BufMut) {
                into.$encode(*value);
            }
        }
    };
}

impl_be!(u32 => put_u32);
impl_be!(i32 => put_i32);
impl_be!(u64 => put_u64);
impl_be!(i64 => put_i64);
impl_be!(u128 => put_u128);
impl_be!(i128 => put_i128);
