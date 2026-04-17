//! Byte-level input/output.

pub mod adapters;
mod bit_size;
mod decoder;
mod encoder;

pub use self::{bit_size::BitSize, decoder::Decoder, encoder::Encoder};

/// Native-endian codec for primitive types.
///
/// 16-bit words are by definition big-endian in Modbus, hence the native-endian codec.
pub struct NativeEndian;

/// Big-endian codec for primitive types composed of multiple words.
pub struct BigEndian;
