#![allow(dead_code)]

use alloc::vec::Vec;
use core::fmt::Debug;

use binrw::{BinWrite, binread};
use bon::bon;

use crate::{error::RequestBuilderError, pdu};

/// Read from 1 to 125 contiguous input registers in a remote device.
pub struct Function;

impl pdu::Function for Function {
    const CODE: u8 = 4;
    type Request = Request;
    type Response = Response;
}

#[must_use]
#[derive(Copy, Clone, Debug, BinWrite)]
#[bw(big, magic = 4_u8)]
pub struct Request {
    /// *Zero-based* address of the first register to read.
    starting_address: u16,

    /// Number of registers to read.
    n_registers: u16,
}

#[bon]
impl Request {
    #[builder]
    pub fn new(starting_address: u16, n_registers: u16) -> Result<Self, RequestBuilderError> {
        if (1..=125).contains(&n_registers) {
            Ok(Self { starting_address, n_registers })
        } else {
            Err(RequestBuilderError::InvalidQuantity(n_registers))
        }
    }
}

#[must_use]
#[binread]
#[br(big, magic = 4_u8)]
#[derive(derive_more::Debug)]
pub struct Response {
    #[br(temp)]
    n_bytes: u8,

    #[br(assert(n_bytes.is_multiple_of(2)), count = n_bytes / 2)]
    pub words: Vec<u16>,
}

impl From<Response> for Vec<u16> {
    fn from(response: Response) -> Self {
        response.words
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use binrw::{BinRead, io::Cursor};

    use super::*;

    #[test]
    fn request_example_ok() {
        const EXPECTED: &[u8] = &[
            0x04, // function code
            0x00, 0x08, // starting address: high, low
            0x00, 0x01, // count: high, low
        ];
        let mut output = Cursor::new(vec![]);
        Request::builder()
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
            0x04, // function code
            0x02, // byte count
            0x00, 0x0A, // value: high, low
        ];
        let response = Response::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert_eq!(response.words, [10]);
    }
}
