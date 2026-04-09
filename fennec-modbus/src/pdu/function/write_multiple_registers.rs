#![allow(dead_code)]

use alloc::vec::Vec;
use core::fmt::Debug;

use binrw::{BinRead, BinWrite};
use bon::bon;

use crate::{error::RequestBuilderError, pdu};

/// Write a block of contiguous registers (1 to 123 registers) in a remote device.
#[derive(Copy, Clone)]
pub struct Function;

impl pdu::Function for Function {
    const CODE: u8 = 16;
    type Request = Request;
    type Response = Response;
}

#[must_use]
#[derive(Clone, Debug, BinWrite)]
#[bw(big, magic = 16_u8)]
pub struct Request {
    starting_address: u16,
    n_registers: u16,
    n_bytes: u8,
    words: Vec<u16>,
}

#[bon]
impl Request {
    #[builder]
    pub fn new(
        /// *Zero-based* address of the first register to read.
        starting_address: u16,
        /// Register values.
        words: Vec<u16>,
    ) -> Result<Self, RequestBuilderError> {
        let n_registers = u16::try_from(words.len())?;
        if (1..=123).contains(&n_registers) {
            let n_bytes = u8::try_from(n_registers * 2).unwrap();
            Ok(Self { starting_address, n_registers, n_bytes, words })
        } else {
            Err(RequestBuilderError::InvalidQuantity(n_registers))
        }
    }
}

#[must_use]
#[derive(Copy, Clone, derive_more::Debug, BinRead)]
#[br(big, magic = 16_u8)]
pub struct Response {
    pub starting_address: u16,
    pub n_registers: u16,
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use binrw::{BinRead, io::Cursor};

    use super::*;

    #[test]
    fn request_example_ok() {
        const EXPECTED: &[u8] = &[
            0x10, // function code
            0x00, 0x01, // starting address: high, low
            0x00, 0x02, // register count: high, low
            0x04, // byte count
            0x00, 0x0A, // first word
            0x01, 0x02, // second word
        ];
        let mut output = Cursor::new(vec![]);
        Request::builder()
            .starting_address(1)
            .words(vec![0x000A, 0x0102])
            .build()
            .unwrap()
            .write(&mut output)
            .unwrap();
        assert_eq!(output.into_inner(), EXPECTED);
    }

    #[test]
    fn response_example_ok() {
        const RESPONSE: &[u8] = &[
            0x10, // function code
            0x00, 0x01, // starting address: high, low
            0x00, 0x02, // register count: high, low
        ];
        let response = Response::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert_eq!(response.starting_address, 1);
        assert_eq!(response.n_registers, 2);
    }
}
