use bytes::Buf;

use crate::Error;

pub trait Decoder<T> {
    fn decode(from: &mut impl Buf) -> Result<T, Error>;
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

impl_be!(u16 => try_get_u16);
impl_be!(i16 => try_get_i16);
impl_be!(u32 => try_get_u32);
impl_be!(i32 => try_get_i32);
impl_be!(u64 => try_get_u64);
impl_be!(i64 => try_get_i64);
impl_be!(u128 => try_get_u128);
impl_be!(i128 => try_get_i128);
