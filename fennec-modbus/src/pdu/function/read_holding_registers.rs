#![allow(dead_code)]

use alloc::vec::Vec;
use core::fmt::Debug;

use binrw::{BinRead, BinWrite};
use bon::bon;

use crate::pdu;

/// Read the contents of a contiguous block of holding registers in a remote device.
#[must_use]
#[derive(Copy, Clone, Debug, BinWrite)]
#[bw(big, magic = 3_u8)]
pub struct Request {
    starting_address: u16,
    n_registers: u16,
}

#[bon]
impl Request {
    #[builder]
    pub fn new(
        /// *Zero-based* address of the first register to read.
        starting_address: u16,
        /// Number of registers to read.
        n_registers: u16,
    ) -> Result<Self, pdu::Error> {
        if (1..=125).contains(&n_registers) {
            Ok(Self { starting_address, n_registers })
        } else {
            Err(pdu::Error::InvalidCount(n_registers.into()))
        }
    }
}

#[must_use]
#[derive(Clone, derive_more::Debug, BinRead)]
#[br(big, magic = 3_u8)]
pub struct Response {
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
            0x03, // function code
            0x00, 0x6B, // starting address: high, low
            0x00, 0x03, // count: high, low
        ];
        let mut output = Cursor::new(vec![]);
        Request::builder()
            .starting_address(107)
            .n_registers(3)
            .build()
            .unwrap()
            .write(&mut output)
            .unwrap();
        assert_eq!(output.into_inner(), EXPECTED);
    }

    #[test]
    fn response_example_ok() {
        const RESPONSE: &[u8] = &[
            0x03, // function code
            0x06, // byte count
            0x02, 0x2B, // value: high, low
            0x00, 0x00, // value: high, low
            0x00, 0x64, // value: high, low
        ];
        let response = Response::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert_eq!(response.words, [555, 0, 100]);
    }
}
