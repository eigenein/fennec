use core::fmt::Debug;

use deku::{DekuRead, DekuWrite};

#[must_use]
#[derive(Copy, Clone, Debug, DekuWrite)]
#[deku(endian = "big")]
pub struct Args;

/// TODO: make a new-type tuple.
#[must_use]
#[derive(Copy, Clone, derive_more::Debug, DekuRead)]
#[deku(endian = "big")]
pub struct Output {
    /// Status of the eight Exception Status outputs.
    ///
    /// The contents of the eight Exception Status outputs are device specific.
    pub output: u8,
}

#[cfg(test)]
mod tests {
    use deku::DekuContainerRead;

    use super::*;

    #[test]
    fn response_example_ok() {
        const RESPONSE: &[u8] = &[
            0x6D, // output
        ];
        let (_, response) = Output::from_bytes((RESPONSE, 0)).unwrap();
        assert_eq!(response.output, 0x6D);
    }
}
