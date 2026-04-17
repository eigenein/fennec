//! Byte-level input/output.

pub mod adapters;
mod bit_size;
mod decoder;
mod encoder;

pub use self::{bit_size::BitSize, decoder::Decoder, encoder::Encoder};

/// Big-endian codec for primitive types.
pub struct BigEndian;
