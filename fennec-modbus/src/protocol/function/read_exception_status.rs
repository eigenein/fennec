use core::fmt::Debug;

use binrw::{BinRead, BinWrite};

#[must_use]
#[derive(Copy, Clone, Debug, BinWrite)]
#[bw(big)]
pub struct Args;

#[must_use]
#[derive(Copy, Clone, derive_more::Debug, BinRead)]
#[br(big)]
pub struct Output {
    /// Status of the eight Exception Status outputs.
    ///
    /// The contents of the eight Exception Status outputs are device specific.
    pub output: u8,
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use binrw::{BinRead, io::Cursor};

    use super::*;

    #[test]
    fn request_example_ok() {
        const EXPECTED: &[u8] = &[];
        let mut output = Cursor::new(vec![]);
        Args.write(&mut output).unwrap();
        assert_eq!(output.into_inner(), EXPECTED);
    }

    #[test]
    fn response_example_ok() {
        const RESPONSE: &[u8] = &[
            0x6D, // output
        ];
        let response = Output::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert_eq!(response.output, 0x6D);
    }
}
