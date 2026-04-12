//! Sans-IO Modbus-over-TCP client decoders.

use crate::{
    protocol::r#struct::Readable,
    tcp::{Error, Header},
};

pub fn decode_header(bytes: &[u8; Header::SIZE]) -> Result<Header, Error> {
    Ok(Header::from_bytes(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_example_ok() {
        let header = decode_header(&[0x15, 0x01, 0x00, 0x00, 0x00, 0x09, 0xFF]).unwrap();
        assert_eq!(header.payload_length(), 8);
        assert_eq!(header.transaction_id, 0x1501);
    }
}
