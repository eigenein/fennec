use alloc::vec::Vec;

use bytes::Buf;

use crate::Error;

pub trait Decode: Sized {
    fn decode_from(buf: &mut impl Buf) -> Result<Self, Error>;
}

impl<T: Decode, const N: usize> Decode for [T; N] {
    fn decode_from(buf: &mut impl Buf) -> Result<Self, Error> {
        // Fix when `array::try_from_fn` becomes stable.
        let mut vec = Vec::with_capacity(N);
        for _ in 0..N {
            vec.push(T::decode_from(buf)?);
        }
        Ok(vec.try_into().unwrap_or_else(|_| unreachable!()))
    }
}

macro_rules! impl_decode {
    ($type:ty => $decode:ident) => {
        impl Decode for $type {
            fn decode_from(buf: &mut impl Buf) -> Result<Self, Error> {
                Ok(buf.$decode()?)
            }
        }
    };
}

impl_decode!(u16 => try_get_u16);
impl_decode!(i16 => try_get_i16);
impl_decode!(u32 => try_get_u32);
impl_decode!(i32 => try_get_i32);
impl_decode!(u64 => try_get_u64);
impl_decode!(i64 => try_get_i64);
impl_decode!(u128 => try_get_u128);
impl_decode!(i128 => try_get_i128);
