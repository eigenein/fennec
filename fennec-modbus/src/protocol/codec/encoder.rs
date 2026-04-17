use bytes::BufMut;

use crate::protocol::codec::NativeEndian;

pub trait Encoder<T: ?Sized> {
    fn encode(value: &T, to: &mut impl BufMut);
}

impl Encoder<u8> for NativeEndian {
    fn encode(value: &u8, to: &mut impl BufMut) {
        to.put_u8(*value);
    }
}

impl Encoder<i8> for NativeEndian {
    fn encode(value: &i8, to: &mut impl BufMut) {
        to.put_i8(*value);
    }
}

impl Encoder<u16> for NativeEndian {
    fn encode(value: &u16, to: &mut impl BufMut) {
        to.put_u16(*value);
    }
}

impl Encoder<i16> for NativeEndian {
    fn encode(value: &i16, to: &mut impl BufMut) {
        to.put_i16(*value);
    }
}

impl Encoder<[u8]> for NativeEndian {
    fn encode(value: &[u8], to: &mut impl BufMut) {
        to.put(value);
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
