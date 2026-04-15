//! Byte-level input/output.

pub mod adapters;

use bytes::{Buf, BufMut};

use crate::protocol::Error;

pub trait Encode {
    /// Encode self into the byte buffer.
    fn encode_into(&self, buf: &mut (impl BufMut + ?Sized));
}

pub trait Decode: Sized {
    /// Decode [`Self`] from the byte buffer.
    fn decode_from(buf: &mut (impl Buf + ?Sized)) -> Result<Self, Error>;
}
