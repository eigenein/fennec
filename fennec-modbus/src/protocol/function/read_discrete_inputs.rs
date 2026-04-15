use bon::bon;
use deku::{DekuContainerRead, DekuRead, DekuWrite};

use crate::protocol;

#[must_use]
#[derive(Copy, Clone, Debug, DekuWrite)]
#[deku(endian = "big")]
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
#[derive(Copy, Clone, derive_more::Debug, DekuRead)]
pub struct Output<S: for<'a> DekuContainerRead<'a>> {
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

    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;

    #[derive(DekuRead)]
    #[deku(endian = "big")]
    struct PackedData {
        #[deku(bits = 8)]
        status_1: u8,

        #[deku(bits = 8)]
        status_2: u8,

        #[deku(bits = 6)]
        status_3: u8,
    }

    #[test]
    fn request_example_ok() {
        const EXPECTED: &[u8] = &[
            0x00, 0xC4, // starting address: high, low
            0x00, 0x16, // count: high, low
        ];
        let bytes =
            Args::builder().starting_address(196).n_inputs(22).build().unwrap().to_bytes().unwrap();
        assert_eq!(bytes, EXPECTED);
    }

    #[test]
    fn response_example_ok() {
        const RESPONSE: &[u8] = &[
            0x03, // byte count
            0xAC, // outputs status 204-197
            0xDB, // outputs status 212-205
            0x35, // outputs status 218-213
        ];

        let (_, response) = Output::<PackedData>::from_bytes((RESPONSE, 0)).unwrap();
        assert_eq!(response.input.status_1, 0xAC);
        assert_eq!(response.input.status_2, 0xDB);
        assert_eq!(response.input.status_3, 0x35);
    }
}
