//! Data model size specifiers.

use crate::protocol::codec::BitSize;

/// Specifies data model alignment requirements for size validation.
pub trait SizeArgument {
    /// Required byte alignment: 1 for bits, 2 for words (registers).
    const BYTE_ALIGNMENT: u8;

    /// The quantity count to write on the wire for a value of type `V`.
    fn quantity_for<V: BitSize>() -> u16;

    /// Assert that the number of bytes in the payload is valid.
    ///
    /// If the value type is too big, the assertion would fire at compile time.
    fn assert_valid_size<V: BitSize, const N_MAX_BYTES: u8>() {
        const {
            assert!(V::N_BYTES >= 1, "value type must be non-empty");
            assert!(V::N_BYTES <= N_MAX_BYTES, "value is too large");
            assert!(
                V::N_BYTES.is_multiple_of(Self::BYTE_ALIGNMENT),
                "value size must be word-aligned for register operations",
            );
        };
    }
}

/// Encode number of bits (coils or discrete inputs).
pub struct Bits;

impl SizeArgument for Bits {
    const BYTE_ALIGNMENT: u8 = 1;

    fn quantity_for<V: BitSize>() -> u16 {
        V::N_BITS
    }
}

/// Encode number of words (registers).
pub struct Words;

impl SizeArgument for Words {
    const BYTE_ALIGNMENT: u8 = 2;

    fn quantity_for<V: BitSize>() -> u16 {
        V::N_WORDS
    }
}
