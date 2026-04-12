use alloc::vec::Vec;
use core::fmt::Debug;

use binrw::{BinRead, BinWrite};
use bon::bon;

use crate::protocol;

/// Read from 1 to 125 contiguous input registers in a remote device.
#[must_use]
pub struct Function;

impl protocol::Function for Function {
    const CODE: u8 = 4;
    type Args = Args;
    type Output = Output;
}

#[must_use]
#[derive(Copy, Clone, Debug, BinWrite)]
#[bw(big)]
pub struct Args {
    starting_address: u16,
    n_registers: u16,
}

#[bon]
impl Args {
    #[builder]
    pub fn new(
        /// *Zero-based* address of the first register to read.
        starting_address: u16,
        /// Number of registers to read.
        n_registers: u16,
    ) -> Result<Self, protocol::Error> {
        if (1..=125).contains(&n_registers) {
            Ok(Self { starting_address, n_registers })
        } else {
            Err(protocol::Error::InvalidCount(n_registers.into()))
        }
    }
}

#[must_use]
#[derive(Clone, derive_more::Debug, BinRead)]
#[br(big)]
pub struct Output {
    pub n_bytes: u8,

    #[br(assert(n_bytes.is_multiple_of(2)), count = n_bytes / 2)]
    pub words: Vec<u16>,
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use binrw::{BinRead, io::Cursor};

    use super::*;

    #[test]
    fn request_example_ok() {
        const EXPECTED: &[u8] = &[
            0x00, 0x08, // starting address: high, low
            0x00, 0x01, // count: high, low
        ];
        let mut output = Cursor::new(vec![]);
        Args::builder()
            .starting_address(8)
            .n_registers(1)
            .build()
            .unwrap()
            .write(&mut output)
            .unwrap();
        assert_eq!(output.into_inner(), EXPECTED);
    }

    #[test]
    fn response_example_ok() {
        const RESPONSE: &[u8] = &[
            0x02, // byte count
            0x00, 0x0A, // value: high, low
        ];
        let response = Output::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert_eq!(response.words, [10]);
    }
}
