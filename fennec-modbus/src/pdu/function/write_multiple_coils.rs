#![allow(dead_code)]

use alloc::vec::Vec;

use binrw::{BinWrite, binread};
use bitvec::vec::BitVec;
use bon::bon;

use crate::{error::RequestBuilderError, pdu};

/// Force each coil in a sequence of coils to either «on» or «off» in a remote device.
pub struct Function;

impl pdu::Function for Function {
    const CODE: u8 = 15;
    type Request = Request;
    type Response = Response;
}

#[must_use]
#[derive(Clone, Debug, BinWrite)]
#[bw(big, magic = 15_u8)]
pub struct Request {
    starting_address: u16,
    n_coils: u16,
    n_bytes: u8,
    values: Vec<u8>,
}

#[bon]
impl Request {
    #[builder]
    pub fn new(
        /// *Zero-based* address of the first coil to write.
        starting_address: u16,
        coils: BitVec<u8>,
    ) -> Result<Self, RequestBuilderError> {
        if (1..=0x07B0).contains(&coils.len()) {
            Ok(Self {
                starting_address,

                #[expect(clippy::cast_possible_truncation)]
                n_coils: coils.len() as u16,

                #[expect(clippy::cast_possible_truncation)]
                n_bytes: coils.len().div_ceil(8) as u8,

                values: coils.into_vec(),
            })
        } else {
            Err(RequestBuilderError::InvalidQuantity(coils.len()))
        }
    }
}

#[must_use]
#[binread]
#[br(big, magic = 15_u8)]
#[derive(derive_more::Debug)]
pub struct Response {
    pub starting_address: u16,
    pub n_coils: u16,
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
            0x0F, // function code
            0x00, 0x13, // starting address
            0x00, 0x0A, // number of coils
            0x02, // number of bytes
            0xCD, 0x01, // packed bits
        ];
        let mut output = Cursor::new(vec![]);
        Request::builder()
            .starting_address(19)
            .coils(bitvec![u8, Lsb0; 1, 0, 1, 1, 0, 0, 1, 1, 1, 0])
            .build()
            .unwrap()
            .write(&mut output)
            .unwrap();
        assert_eq!(output.into_inner(), EXPECTED);
    }

    #[test]
    fn response_example_ok() {
        const RESPONSE: &[u8] = &[
            0x0F, // function code
            0x00, 0x13, // starting address: low, high
            0x00, 0x0A, // number of coils: low, high
        ];

        let response = Response::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert_eq!(response.starting_address, 19);
        assert_eq!(response.n_coils, 10);
    }
}
