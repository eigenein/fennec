pub mod exception;
mod function;
mod response;

pub use self::{function::Function, response::Response};

/// Modbus data model.
///
/// > The distinctions between inputs and outputs, and between bit-addressable and word-
/// > addressable data items, do not imply any application behavior. It is perfectly acceptable, and
/// > very common, to regard all four tables as overlaying one another, if this is the most natural
/// > interpretation on the target machine in question.
/// >
/// > For each of the primary tables, the protocol allows individual selection of 65536 data items,
/// > and the operations of read or write of those items are designed to span multiple consecutive
/// > data items up to a data size limit which is dependent on the transaction function code.
/// >
/// > **The pre-mapping between the MODBUS data model and the device application is totally
/// > vendor device specific.**
pub enum Table {
    /// Read-only single bit.
    DiscreteInput,

    /// Read-write single bit.
    Coil,

    /// Read-only 16-bit word.
    InputRegister,

    /// Read-write 16-bit word.
    HoldingRegister,
}
