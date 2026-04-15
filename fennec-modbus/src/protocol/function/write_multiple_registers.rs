use alloc::vec::Vec;
use core::fmt::Debug;

use bon::bon;
use deku::{DekuRead, DekuWrite};

use crate::protocol;

#[must_use]
#[derive(Clone, Debug, DekuWrite)]
#[deku(endian = "big")]
pub struct Args {
    starting_address: u16,
    n_registers: u16,
    n_bytes: u8,
    words: Vec<u16>,
}

#[bon]
impl Args {
    #[builder]
    pub fn new(
        /// *Zero-based* address of the first register to read.
        starting_address: u16,
        /// Register values.
        words: Vec<u16>,
    ) -> Result<Self, protocol::Error> {
        let n_registers = words.len();
        if (1..=123).contains(&n_registers) {
            let n_registers = u16::try_from(n_registers).unwrap();
            let n_bytes = u8::try_from(n_registers * 2).unwrap();
            Ok(Self { starting_address, n_registers, n_bytes, words })
        } else {
            Err(protocol::Error::InvalidCount(n_registers))
        }
    }
}

#[must_use]
#[derive(Copy, Clone, derive_more::Debug, DekuRead)]
#[deku(endian = "big")]
pub struct Output {
    pub starting_address: u16,
    pub n_registers: u16,
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;

    #[test]
    fn request_example_ok() {
        const EXPECTED: &[u8] = &[
            0x00, 0x01, // starting address: high, low
            0x00, 0x02, // register count: high, low
            0x04, // byte count
            0x00, 0x0A, // first word
            0x01, 0x02, // second word
        ];
        let bytes = Args::builder()
            .starting_address(1)
            .words(vec![0x000A, 0x0102])
            .build()
            .unwrap()
            .to_bytes()
            .unwrap();
        assert_eq!(bytes, EXPECTED);
    }

    #[test]
    fn response_example_ok() {
        const RESPONSE: &[u8] = &[
            0x00, 0x01, // starting address: high, low
            0x00, 0x02, // register count: high, low
        ];
        let (_, output) = Output::from_bytes((RESPONSE, 0)).unwrap();
        assert_eq!(output.starting_address, 1);
        assert_eq!(output.n_registers, 2);
    }
}
