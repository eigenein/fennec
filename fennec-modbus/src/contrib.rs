//! Modbus client wrappers for different devices.

use bytes::Buf;

use crate::{
    Error,
    protocol::codec::{BigEndian, BitSize, Decoder, NativeEndian},
};

pub mod mq2200;

macro_rules! impl_new_type {
    ($target:ident => $codec:ty, $inner:ty) => {
        #[derive(Copy, Clone, Debug)]
        pub struct $target(pub $inner);

        impl BitSize for $target {
            const N_BITS: u16 = <$inner as BitSize>::N_BITS;
        }

        impl Decoder<$target> for $codec {
            fn decode(from: &mut impl Buf) -> Result<$target, Error> {
                <$codec>::decode(from).map($target)
            }
        }
    };
}

impl_new_type!(Percentage => NativeEndian, u16);
impl_new_type!(DecawattHours => NativeEndian, u16);
impl_new_type!(Watts => BigEndian, i32);
