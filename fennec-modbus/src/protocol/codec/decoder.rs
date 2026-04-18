use bytes::Buf;

use crate::{Error, protocol::codec::NativeEndian};

pub trait Decoder<T> {
    fn decode(from: &mut impl Buf) -> Result<T, Error>;
}

impl Decoder<u16> for NativeEndian {
    fn decode(from: &mut impl Buf) -> Result<u16, Error> {
        Ok(from.try_get_u16()?)
    }
}

impl Decoder<i16> for NativeEndian {
    fn decode(from: &mut impl Buf) -> Result<i16, Error> {
        Ok(from.try_get_i16()?)
    }
}

macro_rules! impl_be {
    ($type:ty => $decode:ident) => {
        impl Decoder<$type> for crate::protocol::codec::BigEndian {
            fn decode(from: &mut impl Buf) -> Result<$type, Error> {
                Ok(from.$decode()?)
            }
        }
    };
}

impl_be!(u32 => try_get_u32);
impl_be!(i32 => try_get_i32);
impl_be!(u64 => try_get_u64);
impl_be!(i64 => try_get_i64);
impl_be!(u128 => try_get_u128);
impl_be!(i128 => try_get_i128);
