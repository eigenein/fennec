//! Byte-level input/output.

pub mod adapters;
mod bit_size;
mod decoder;
mod encoder;

pub use self::{bit_size::BitSize, decoder::Decoder, encoder::Encoder};

/// Native-endian codec for primitive types composed of one word.
pub struct Word;

/// Big-endian codec for primitive types composed of multiple words.
pub struct BigEndian;

/// Codec for complex structures composed of multiple words.
pub struct Struct;
