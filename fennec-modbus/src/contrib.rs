//! Modbus client wrappers for different devices.

use bytes::Buf;

use crate::{
    Error,
    protocol::codec::{BigEndian, Decoder, NativeEndian},
};

pub mod mq2200;

macro_rules! impl_new_type {
    ($target:path => $codec:ty) => {
        impl Decoder<$target> for $codec {
            fn decode(from: &mut impl Buf) -> Result<$target, Error> {
                <$codec>::decode(from).map($target)
            }
        }
    };
}

#[derive(Copy, Clone)]
pub struct Percentage(pub u16);

impl_new_type!(Percentage => NativeEndian);

#[derive(Copy, Clone)]
pub struct DecawattHours(pub u16);

impl_new_type!(DecawattHours => NativeEndian);

#[derive(Copy, Clone)]
pub struct Watts(pub i32);

impl_new_type!(Watts => BigEndian);
