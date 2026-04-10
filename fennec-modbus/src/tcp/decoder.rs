//! Sans-IO Modbus-over-TCP client decoders.

use binrw::{BinRead, io::Cursor};

use crate::{
    protocol,
    tcp::{Error, Header},
};

pub fn decode_header(bytes: &[u8; Header::SIZE]) -> Result<Header, Error> {
    Ok(Header::read_be(&mut Cursor::new(bytes))?)
}

pub fn decode_payload<T: for<'a> BinRead<Args<'a> = ()>>(bytes: &[u8]) -> Result<T, Error> {
    Ok(protocol::Response::<T>::read_be(&mut Cursor::new(bytes))?.into_result()?)
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
