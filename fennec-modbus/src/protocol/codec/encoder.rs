use alloc::vec::Vec;

use bytes::BufMut;

pub trait Encode {
    fn encode_to(&self, buf: &mut impl BufMut);

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.encode_to(&mut bytes);
        bytes
    }
}

impl<T: Encode, const N: usize> Encode for [T; N] {
    fn encode_to(&self, buf: &mut impl BufMut) {
        for item in self {
            item.encode_to(buf);
        }
    }
}

macro_rules! impl_encode {
    ($type:ty => $encode:ident) => {
        impl Encode for $type {
            fn encode_to(&self, buf: &mut impl BufMut) {
                buf.$encode(*self);
            }
        }
    };
}

impl_encode!(u16 => put_u16);
impl_encode!(i16 => put_i16);
impl_encode!(u32 => put_u32);
impl_encode!(i32 => put_i32);
impl_encode!(u64 => put_u64);
impl_encode!(i64 => put_i64);
impl_encode!(u128 => put_u128);
impl_encode!(i128 => put_i128);
