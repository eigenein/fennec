#![allow(dead_code)]

use alloc::vec::Vec;

use binrw::{BinWrite, binread};
use bitvec::prelude::BitVec;
use bon::bon;

use crate::{error::RequestBuilderError, pdu};

/// Read from 1 to 2000 contiguous status of coils in a remote device.
pub struct Function;

impl pdu::Function for Function {
    const CODE: u8 = 1;
    type Request = Request;
    type Response = Response;
}

#[must_use]
#[derive(Copy, Clone, Debug, BinWrite)]
#[bw(big, magic = 1_u8)]
pub struct Request {
    /// *Zero-based* address of the first coil to read.
    starting_address: u16,

    /// Number of coils to read.
    n_coils: u16,
}

#[bon]
impl Request {
    #[builder]
    pub fn new(starting_address: u16, n_coils: u16) -> Result<Self, RequestBuilderError> {
        if (1..=2000).contains(&n_coils) {
            Ok(Self { starting_address, n_coils })
        } else {
            Err(RequestBuilderError::InvalidQuantity(n_coils.into()))
        }
    }
}

#[must_use]
#[binread]
#[br(big, magic = 1_u8)]
#[derive(derive_more::Debug)]
pub struct Response {
    #[br(temp)]
    n_bytes: u8,

    /// The coils in the response message are packed as one coil per bit of the data field.
    ///
    /// The LSB of the first data byte contains the output addressed in the query.
    /// The other coils follow toward the high order end of this byte, and from low order to high order in subsequent bytes.
    #[br(count = n_bytes)]
    coils: Vec<u8>,
}

impl From<Response> for BitVec<u8> {
    fn from(response: Response) -> Self {
        BitVec::from_vec(response.coils)
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use binrw::{BinRead, io::Cursor};
    use bitvec::prelude::*;

    use super::*;

    #[test]
    fn request_example_ok() {
        const EXPECTED: &[u8] = &[
            0x01, // function code
            0x00, 0x13, // starting address: high, low
            0x00, 0x13, // count: high, low
        ];
        let mut output = Cursor::new(vec![]);
        Request::builder()
            .starting_address(19)
            .n_coils(19)
            .build()
            .unwrap()
            .write(&mut output)
            .unwrap();
        assert_eq!(output.into_inner(), EXPECTED);
    }

    #[test]
    fn response_example_ok() {
        const RESPONSE: &[u8] = &[
            0x01, // function code
            0x03, // byte count
            0xCD, // outputs status 27-20
            0x6B, // outputs status 35-28
            0x05, // outputs status 38-36
        ];

        let response = Response::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert_eq!(
            BitVec::from(response),
            bitvec![u8, Lsb0; 1, 0, 1, 1, 0, 0, 1, 1, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0],
        );
    }
}
