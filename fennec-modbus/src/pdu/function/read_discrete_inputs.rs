#![allow(dead_code)]

use alloc::vec::Vec;

use binrw::{BinWrite, binread};
use bon::bon;

use crate::{error::RequestBuilderError, pdu};

/// Read from 1 to 2000 contiguous status of discrete inputs in a remote device.
pub struct Function;

impl pdu::Function for Function {
    const CODE: u8 = 2;
    type Request = Request;
    type Response = Response;
}

#[must_use]
#[derive(Copy, Clone, Debug, BinWrite)]
#[bw(big, magic = 2_u8)]
pub struct Request {
    /// *Zero-based* address of the first coil to read.
    starting_address: u16,

    /// Number of inputs to read.
    n_inputs: u16,
}

#[bon]
impl Request {
    #[builder]
    pub fn new(starting_address: u16, n_inputs: u16) -> Result<Self, RequestBuilderError> {
        if (1..=2000).contains(&n_inputs) {
            Ok(Self { starting_address, n_inputs })
        } else {
            Err(RequestBuilderError::InvalidQuantity(n_inputs))
        }
    }
}

#[must_use]
#[binread]
#[br(big, magic = 2_u8)]
#[derive(derive_more::Debug)]
pub struct Response {
    #[br(temp)]
    n_bytes: u8,

    /// The coils in the response message are packed as one coil per bit of the data field.
    ///
    /// The LSB of the first data byte contains the output addressed in the query.
    /// The other coils follow toward the high order end of this byte, and from low order to high order in subsequent bytes.
    #[br(count = n_bytes)]
    pub outputs: Vec<u8>,
}

impl From<Response> for Vec<u8> {
    fn from(response: Response) -> Self {
        response.outputs
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
            0x02, // function code
            0x00, 0xC4, // starting address: high, low
            0x00, 0x16, // count: high, low
        ];
        let mut output = Cursor::new(vec![]);
        Request::builder()
            .starting_address(196)
            .n_inputs(22)
            .build()
            .unwrap()
            .write(&mut output)
            .unwrap();
        assert_eq!(output.into_inner(), EXPECTED);
    }

    #[test]
    fn response_example_ok() {
        const RESPONSE: &[u8] = &[
            0x02, // function code
            0x03, // byte count
            0xAC, // outputs status 204-197
            0xDB, // outputs status 212-205
            0x35, // outputs status 218-213
        ];

        let response = Response::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert_eq!(response.outputs, [0xAC, 0xDB, 0x35]);
    }
}
