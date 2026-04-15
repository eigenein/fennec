use bon::bon;
use deku::{DekuContainerRead, DekuRead, DekuWrite};

use crate::protocol;

#[must_use]
#[derive(Copy, Clone, derive_more::Debug, DekuWrite)]
#[deku(endian = "big")]
pub struct Args {
    starting_address: u16,
    n_coils: u16,
}

#[bon]
impl Args {
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
#[derive(Copy, Clone, derive_more::Debug, DekuRead)]
pub struct Output<S: for<'a> DekuContainerRead<'a>> {
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

        #[deku(bits = 3)]
        status_3: u8,
    }

    #[test]
    fn request_example_ok() {
        const EXPECTED: &[u8] = &[
            0x00, 0x13, // starting address: high, low
            0x00, 0x13, // count: high, low
        ];
        let bytes =
            Args::builder().starting_address(19).n_coils(19).build().unwrap().to_bytes().unwrap();
        assert_eq!(bytes, EXPECTED);
    }

    #[test]
    fn response_example_ok() {
        const RESPONSE: &[u8] = &[
            0x03, // byte count
            0xCD, // outputs status 27-20
            0x6B, // outputs status 35-28
            0x05, // outputs status 38-36
        ];

        let (_, response) = Output::<PackedData>::from_bytes((RESPONSE, 0)).unwrap();
        assert_eq!(response.coils.status_1, 0xCD);
        assert_eq!(response.coils.status_2, 0x6B);
        assert_eq!(response.coils.status_3, 0x05);
    }
}
