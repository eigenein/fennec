#![allow(dead_code)]

use core::marker::PhantomData;

use binrw::{BinRead, BinWrite, binread};
use bon::bon;

use crate::{error::RequestBuilderError, pdu};

/// Read from 1 to 2000 contiguous status of discrete inputs in a remote device.
#[derive(Copy, Clone)]
pub struct Function<S>(PhantomData<S>);

impl<S: for<'a> BinRead<Args<'a> = ()> + Send + 'static> pdu::Function for Function<S> {
    const CODE: u8 = 2;
    type Request = Request;
    type Response = Response<S>;
}

#[must_use]
#[derive(Copy, Clone, Debug, BinWrite)]
#[bw(big, magic = 2_u8)]
pub struct Request {
    /// *Zero-based* address of the first input to read.
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
pub struct Response<S: for<'a> BinRead<Args<'a> = ()>> {
    #[br(temp)]
    n_bytes: u8,

    /// The discrete inputs in the response message are packed as one input per bit of the data field.
    ///
    /// The LSB of the first data byte contains the input addressed in the query.
    /// The other inputs follow toward the high order end of this byte, and from low order to high order in subsequent bytes.
    pub inputs: S,
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use binrw::{BinRead, io::Cursor};
    use modular_bitfield::prelude::*;

    use super::*;

    #[bitfield]
    #[derive(BinRead)]
    #[br(map = Self::from_bytes)]
    struct PackedData {
        status_1: B8,
        status_2: B8,
        status_3: B6,

        #[skip]
        __: B2,
    }

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

        let response = Response::<PackedData>::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert_eq!(response.inputs.status_1(), 0xAC);
        assert_eq!(response.inputs.status_2(), 0xDB);
        assert_eq!(response.inputs.status_3(), 0x35);
    }
}
