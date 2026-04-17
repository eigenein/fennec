//! Modbus client wrappers for different devices.

use bytes::Buf;

use crate::{
    Error,
    protocol::codec::{BigEndian, BitSize, Decoder, NativeEndian},
};

pub mod mq2200;

macro_rules! impl_new_type {
    ($target:ident => $codec:ty, $inner:ty) => {
        impl BitSize for $target<$inner> {
            const N_BITS: u16 = <$inner as BitSize>::N_BITS;
        }

        impl Decoder<$target<$inner>> for $codec {
            fn decode(from: &mut impl Buf) -> Result<$target<$inner>, Error> {
                <$codec>::decode(from).map($target)
            }
        }
    };
}

#[derive(Copy, Clone, Debug)]
pub struct Percentage<T>(pub T);

impl_new_type!(Percentage => NativeEndian, u8);
impl_new_type!(Percentage => NativeEndian, u16);

#[derive(Copy, Clone, Debug)]
pub struct DecawattHours<T>(pub T);

impl_new_type!(DecawattHours => NativeEndian, u16);

#[derive(Copy, Clone, Debug)]
pub struct Watts<T>(pub T);

impl_new_type!(Watts => NativeEndian, u16);
impl_new_type!(Watts => BigEndian, i32);
