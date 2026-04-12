use binrw::{BinRead, BinWrite};
use bon::bon;

use crate::{protocol, protocol::r#struct::Readable};

#[must_use]
#[derive(Copy, Clone, Debug, BinWrite)]
#[bw(big)]
pub struct Args {
    /// *Zero-based* address of the first input to read.
    starting_address: u16,

    /// Number of inputs to read.
    n_inputs: u16,
}

#[bon]
impl Args {
    #[builder]
    pub fn new(starting_address: u16, n_inputs: u16) -> Result<Self, protocol::Error> {
        if (1..=2000).contains(&n_inputs) {
            Ok(Self { starting_address, n_inputs })
        } else {
            Err(protocol::Error::InvalidCount(n_inputs.into()))
        }
    }
}

#[must_use]
#[derive(Copy, Clone, derive_more::Debug, BinRead)]
#[br(big)]
pub struct Output<S: Readable> {
    pub n_bytes: u8,

    /// The discrete inputs in the response message are packed as one input per bit of the data field.
    ///
    /// The LSB of the first data byte contains the input addressed in the query.
    /// The other inputs follow toward the high order end of this byte, and from low order to high order in subsequent bytes.
    ///
    /// *Extra data at the end is ignored.*
    pub input: S,
}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]

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
            0x00, 0xC4, // starting address: high, low
            0x00, 0x16, // count: high, low
        ];
        let mut output = Cursor::new(vec![]);
        Args::builder()
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
            0x03, // byte count
            0xAC, // outputs status 204-197
            0xDB, // outputs status 212-205
            0x35, // outputs status 218-213
        ];

        let response = Output::<PackedData>::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert_eq!(response.input.status_1(), 0xAC);
        assert_eq!(response.input.status_2(), 0xDB);
        assert_eq!(response.input.status_3(), 0x35);
    }
}
