use bytes::{Buf, BufMut};

use crate::{
    Error,
    protocol::codec::{BitSize, Decode, Encode},
};

// TODO: make endianness a type parameter.
macro_rules! impl_new_type {
    ($target:ident => $inner:ty) => {
        impl BitSize for $target<$inner> {
            const N_BITS: u16 = <$inner as BitSize>::N_BITS;
            const N_BYTES: u8 = <$inner as BitSize>::N_BYTES;
            const N_WORDS: u16 = <$inner as BitSize>::N_WORDS;
        }

        impl Decode for $target<$inner> {
            fn decode_from(buf: &mut impl Buf) -> Result<Self, Error> {
                <$inner>::decode_from(buf).map(Self)
            }
        }

        impl Encode for $target<$inner> {
            fn encode_to(&self, buf: &mut impl BufMut) {
                self.0.encode_to(buf);
            }
        }
    };
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Percentage<T>(pub T);

impl_new_type!(Percentage => u16);

/// [Decawatt][1]-hours, 1 daWh is equal to 10 [Wh][2].
///
/// [1]: https://en.wiktionary.org/wiki/decawatt
/// [2]: https://en.wiktionary.org/wiki/watt-hour
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DecawattHours<T>(pub T);

impl_new_type!(DecawattHours => u16);
impl_new_type!(DecawattHours => u32);

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Watts<T>(pub T);

impl_new_type!(Watts => u16);
impl_new_type!(Watts => i32);
