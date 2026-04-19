//! Modbus client wrappers for different devices.

use bytes::{Buf, BufMut};

use crate::{
    Error,
    protocol::codec::{BitSize, Decode, Encode},
};

pub mod mq2200;

// TODO: make endianness a type parameter.
macro_rules! impl_new_type {
    ($target:ident => $inner:ty) => {
        impl BitSize for $target<$inner> {
            const N_BITS: u16 = <$inner as BitSize>::N_BITS;
        }

        impl Decode for $target<$inner> {
            fn decode(from: &mut impl Buf) -> Result<Self, Error> {
                <$inner>::decode(from).map(Self)
            }
        }

        impl Encode for $target<$inner> {
            fn encode(&self, to: &mut impl BufMut) {
                self.0.encode(to);
            }
        }
    };
}

#[derive(Copy, Clone, Debug)]
pub struct Percentage<T>(pub T);

impl_new_type!(Percentage => u16);

#[derive(Copy, Clone, Debug)]
pub struct DecawattHours<T>(pub T);

impl_new_type!(DecawattHours => u16);

#[derive(Copy, Clone, Debug)]
pub struct Watts<T>(pub T);

impl_new_type!(Watts => u16);
impl_new_type!(Watts => i32);
