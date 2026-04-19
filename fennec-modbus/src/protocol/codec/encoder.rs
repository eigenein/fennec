use bytes::BufMut;

pub trait Encode {
    fn encode(&self, to: &mut impl BufMut);
}

macro_rules! impl_be {
    ($type:ty => $encode:ident) => {
        impl Encode for $type {
            fn encode(&self, to: &mut impl BufMut) {
                to.$encode(*self);
            }
        }
    };
}

impl_be!(u16 => put_u16);
impl_be!(i16 => put_i16);
impl_be!(u32 => put_u32);
impl_be!(i32 => put_i32);
impl_be!(u64 => put_u64);
impl_be!(i64 => put_i64);
impl_be!(u128 => put_u128);
impl_be!(i128 => put_i128);
