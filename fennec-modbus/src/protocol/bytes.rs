//! Byte-level input/output.

pub mod adapters;

use alloc::vec::Vec;

use bytes::{Buf, BufMut};

use crate::protocol::Error;

pub trait BitSize {
    /// Number of bits occupied by the value.
    const N_BITS: usize;

    /// Number of whole bytes occupied by the value.
    const N_BYTES: usize = Self::N_BITS.div_ceil(8);
}

pub trait Encode {
    /// Encode self into the byte buffer.
    fn encode_into(&self, buf: &mut impl BufMut);

    fn encode_into_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.encode_into(&mut bytes);
        bytes
    }
}

impl Encode for () {
    fn encode_into(&self, _buf: &mut impl BufMut) {}
}

impl<const N: usize> Encode for [u8; N] {
    fn encode_into(&self, buf: &mut impl BufMut) {
        buf.put(&self[..]);
    }
}

pub trait Decode: Sized {
    /// Decode [`Self`] from the byte buffer.
    fn decode_from(buf: &mut impl Buf) -> Result<Self, Error>;
}

macro_rules! impl_primitive {
    ($type:ty, $n_bits:literal, $encode:ident, $decode:ident) => {
        impl BitSize for $type {
            const N_BITS: usize = $n_bits;
        }

        impl Encode for $type {
            fn encode_into(&self, buf: &mut impl BufMut) {
                buf.$encode(*self);
            }
        }

        impl Decode for $type {
            fn decode_from(buf: &mut impl Buf) -> Result<Self, Error> {
                buf.$decode().map_err(Error::from)
            }
        }
    };
}

impl_primitive!(i8, 8, put_i8, try_get_i8);
impl_primitive!(u8, 8, put_u8, try_get_u8);
impl_primitive!(u16, 16, put_u16, try_get_u16);
impl_primitive!(i16, 16, put_i16, try_get_i16);
impl_primitive!(u32, 32, put_u32, try_get_u32);
impl_primitive!(i32, 32, put_i32, try_get_i32);
impl_primitive!(u64, 64, put_u64, try_get_u64);
impl_primitive!(i64, 64, put_i64, try_get_i64);
impl_primitive!(u128, 128, put_u128, try_get_u128);
impl_primitive!(i128, 128, put_i128, try_get_i128);
