use bytes::BufMut;

use crate::protocol::{
    Address,
    codec::{BitSize, Encode},
    function::{read_multiple, size_argument, size_argument::SizeArgument, write_multiple},
};

/// Address range for the reading operation along with address range and value for the writing operation.
///
/// # Example
///
/// ```rust
/// use fennec_modbus::protocol::{codec::Encode, function::read_write_multiple::Args};
///
/// // Read six registers starting at register 4, and to write three
/// // registers starting at register 15 (Modbus spec §6.17 example).
/// assert_eq!(
///     Args::<_, [u16; 6], _, _>::new(3, 14, [0x00FF_u16, 0x00FF, 0x00FF]).to_bytes(),
///     [
///         0x00, 0x03, // read starting address
///         0x00, 0x06, // quantity of registers to read
///         0x00, 0x0E, // write starting address
///         0x00, 0x03, // quantity to write
///         0x06, // write byte count
///         0x00, 0xFF, // register 1
///         0x00, 0xFF, // register 2
///         0x00, 0xFF, // register 3
///     ]
/// );
/// ```
#[must_use]
pub struct Args<ReadAddress, ReadValue, WriteAddress, WriteValue> {
    read: read_multiple::Args<ReadAddress, ReadValue, size_argument::Words>,
    write: write_multiple::Args<WriteAddress, WriteValue, size_argument::Words>,
}

impl<ReadAddress, ReadValue: BitSize, WriteAddress, WriteValue>
    Args<ReadAddress, ReadValue, WriteAddress, WriteValue>
{
    pub const fn new(
        read_address: ReadAddress,
        write_address: WriteAddress,
        write_value: WriteValue,
    ) -> Self {
        Self {
            read: read_multiple::Args::new(read_address),
            write: write_multiple::Args::new(write_address, write_value),
        }
    }
}

impl<ReadAddress, ReadValue, WriteAddress, WriteValue> Encode
    for Args<ReadAddress, ReadValue, WriteAddress, WriteValue>
where
    ReadAddress: Address,
    ReadValue: BitSize,
    WriteAddress: Address,
    WriteValue: BitSize + Encode,
{
    fn encode_to(&self, buf: &mut impl BufMut) {
        size_argument::Words::assert_valid_size::<WriteValue, 242>();
        self.read.encode_to(buf);
        self.write.encode_to(buf);
    }
}
