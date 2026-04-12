use alloc::vec::Vec;

use binrw::{BinRead, BinWrite, io::Cursor};
use bon::bon;

use crate::{protocol, protocol::r#struct::Writable};

/// Force each coil in a sequence of coils to either «on» or «off» in a remote device.
#[must_use]
pub struct Function;

impl protocol::Function for Function {
    const CODE: u8 = 15;
    type Args = Args;
    type Output = Output;
}

#[must_use]
#[derive(Clone, Debug, BinWrite)]
#[bw(big)]
pub struct Args {
    starting_address: u16,
    n_coils: u16,
    n_bytes: u8,
    coil_bytes: Vec<u8>,
}

#[bon]
impl Args {
    #[builder]
    pub fn new<S: Writable>(
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
            let coil_bytes = {
                let mut buffer = Cursor::new(Vec::new());
                coils.write_be(&mut buffer)?;
                buffer.into_inner()
            };
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
#[derive(Copy, Clone, derive_more::Debug, BinRead)]
#[br(big)]
pub struct Output {
    pub starting_address: u16,
    pub n_coils: u16,
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use binrw::{BinRead, io::Cursor};
    use modular_bitfield::prelude::*;

    use super::*;

    #[bitfield]
    #[derive(Copy, Clone, BinWrite)]
    #[bw(map = |&it| Self::into_bytes(it))]
    struct PackedData {
        #[allow(dead_code)]
        status_1: B8,

        #[allow(dead_code)]
        status_2: B2,

        #[skip]
        __: B6,
    }

    #[test]
    fn request_example_ok() {
        const EXPECTED: &[u8] = &[
            0x00, 0x13, // starting address
            0x00, 0x0A, // number of coils
            0x02, // number of bytes
            0xCD, 0x01, // packed bits
        ];
        let mut output = Cursor::new(vec![]);
        Args::builder()
            .starting_address(19)
            .n_coils(10)
            .coils(PackedData::new().with_status_1(0xCD).with_status_2(1))
            .build()
            .unwrap()
            .write(&mut output)
            .unwrap();
        assert_eq!(output.into_inner(), EXPECTED);
    }

    #[test]
    fn response_example_ok() {
        const RESPONSE: &[u8] = &[
            0x00, 0x13, // starting address: low, high
            0x00, 0x0A, // number of coils: low, high
        ];

        let response = Output::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert_eq!(response.starting_address, 19);
        assert_eq!(response.n_coils, 10);
    }
}
