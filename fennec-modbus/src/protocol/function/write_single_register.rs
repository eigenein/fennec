use binrw::{BinRead, BinWrite};
use bon::Builder;

#[must_use]
#[derive(Builder, Copy, Clone, Debug, BinRead, BinWrite)]
#[brw(big)]
pub struct Payload {
    /// *Zero-based* address of the register to write.
    address: u16,

    value: u16,
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use binrw::{BinRead, io::Cursor};

    use super::*;

    const PAYLOAD: &[u8] = &[
        0x00, 0x01, // address: high, low
        0x00, 0x03, // output value: high, low
    ];

    #[test]
    fn request_example_ok() {
        let mut output = Cursor::new(vec![]);
        Payload::builder().address(1).value(3).build().write(&mut output).unwrap();
        assert_eq!(output.into_inner(), PAYLOAD);
    }

    #[test]
    fn response_example_ok() {
        let response = Payload::read(&mut Cursor::new(PAYLOAD)).unwrap();
        assert_eq!(response.address, 1);
        assert_eq!(response.value, 3);
    }
}
