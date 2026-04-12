//! Convenience traits to read and write low-level protocol structures.

use alloc::vec::Vec;

use binrw::{
    BinRead,
    BinReaderExt,
    BinWrite,
    io::{Cursor, Seek, Write},
};

use crate::protocol::Error;

pub trait Readable: for<'a> BinRead<Args<'a> = ()> {
    /// Read [`Self`] from the slice.
    fn from_bytes(bytes: &[u8]) -> Result<Self, Error>;
}

impl<T: for<'a> BinRead<Args<'a> = ()>> Readable for T {
    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Cursor::new(bytes).read_be()?)
    }
}

pub trait Writable: for<'a> BinWrite<Args<'a> = ()> {
    /// Write [`Self`] into a [`Vec`] and return it.
    fn to_bytes(&self) -> Result<Vec<u8>, Error>;

    /// Write [`Self`] to the writer.
    fn write_to(&self, writer: &mut (impl Write + Seek)) -> Result<(), Error>;
}

impl<T: for<'a> BinWrite<Args<'a> = ()>> Writable for T {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut cursor = Cursor::new(Vec::new());
        self.write_to(&mut cursor)?;
        Ok(cursor.into_inner())
    }

    fn write_to(&self, writer: &mut (impl Write + Seek)) -> Result<(), Error> {
        self.write_be(writer)?;
        Ok(())
    }
}
