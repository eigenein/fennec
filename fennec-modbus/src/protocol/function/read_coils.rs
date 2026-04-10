#![allow(dead_code)]

use binrw::{BinRead, BinWrite};
use bon::bon;

use crate::protocol;

/// Read from 1 to 2000 contiguous status of coils in a remote device.
#[must_use]
#[derive(Copy, Clone, derive_more::Debug, BinWrite)]
#[bw(big, magic = 1_u8)]
pub struct Request {
    starting_address: u16,
    n_coils: u16,
}

#[bon]
impl Request {
    #[builder]
    pub fn new(
        /// *Zero-based* address of the first coil to read.
        starting_address: u16,
        /// Number of coils to read.
        n_coils: u16,
    ) -> Result<Self, protocol::Error> {
        if (1..=2000).contains(&n_coils) {
            Ok(Self { starting_address, n_coils })
        } else {
            Err(protocol::Error::InvalidCount(n_coils.into()))
        }
    }
}

#[must_use]
#[derive(Copy, Clone, derive_more::Debug, BinRead)]
#[br(big, magic = 1_u8)]
pub struct Response<S: for<'a> BinRead<Args<'a> = ()>> {
    pub n_bytes: u8,

    /// The coils in the response message are packed as one coil per bit of the data field.
    ///
    /// The LSB of the first data byte contains the output addressed in the query.
    /// The other coils follow toward the high order end of this byte, and from low order to high order in subsequent bytes.
    ///
    /// *Extra data at the end is ignored.*
    pub coils: S,
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
        status_3: B3,

        #[skip]
        __: B5,
    }

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

        let response = Response::<PackedData>::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert_eq!(response.coils.status_1(), 0xCD);
        assert_eq!(response.coils.status_2(), 0x6B);
        assert_eq!(response.coils.status_3(), 0x05);
    }
}
