use alloc::vec::Vec;

use bon::bon;
use deku::{DekuContainerWrite, DekuRead, DekuWrite};

use crate::protocol;

#[must_use]
#[derive(Clone, Debug, DekuWrite)]
#[deku(endian = "big")]
pub struct Args {
    starting_address: u16,
    n_coils: u16,
    n_bytes: u8,
    coil_bytes: Vec<u8>,
}

#[bon]
impl Args {
    #[builder]
    pub fn new<S: DekuContainerWrite>(
        /// *Zero-based* address of the first coil to write.
        starting_address: u16,
        /// Number of coils to write.
        n_coils: u16,
        /// Coil settings.
        coils: S,
    ) -> Result<Self, protocol::Error> {
        if (1..=0x07B0).contains(&n_coils) {
            // Infallible since `n_coils` is verified:
            let n_bytes = u8::try_from(n_coils.div_ceil(8)).unwrap();
            let coil_bytes = coils.to_bytes()?;
            if coil_bytes.len() == n_bytes.into() {
                Ok(Self { starting_address, n_coils, n_bytes, coil_bytes })
            } else {
                Err(protocol::Error::CoilNumberMismatch {
                    n_expected_bytes: n_bytes.into(),
                    n_actual_bytes: coil_bytes.len(),
                })
            }
        } else {
            Err(protocol::Error::InvalidCount(n_coils.into()))
        }
    }
}

#[must_use]
#[derive(Copy, Clone, derive_more::Debug, DekuRead)]
#[deku(endian = "big")]
pub struct Output {
    pub starting_address: u16,
    pub n_coils: u16,
}

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;

    #[derive(Copy, Clone, DekuWrite)]
    struct PackedData {
        #[deku(bits = 8)]
        status_1: u8,

        #[deku(bits = 2)]
        status_2: u8,
    }

    #[test]
    fn request_example_ok() {
        const EXPECTED: &[u8] = &[
            0x00, 0x13, // starting address
            0x00, 0x0A, // number of coils
            0x02, // number of bytes
            0xCD, 0x01, // packed bits
        ];
        let bytes = Args::builder()
            .starting_address(19)
            .n_coils(10)
            .coils(PackedData { status_1: 0xCD, status_2: 1 })
            .build()
            .unwrap()
            .to_bytes()
            .unwrap();
        assert_eq!(bytes, EXPECTED);
    }

    #[test]
    fn response_example_ok() {
        let (_, output) = Output::from_bytes((
            &[
                0x00, 0x13, // starting address: low, high
                0x00, 0x0A, // number of coils: low, high
            ],
            0,
        ))
        .unwrap();
        assert_eq!(output.starting_address, 19);
        assert_eq!(output.n_coils, 10);
    }
}
