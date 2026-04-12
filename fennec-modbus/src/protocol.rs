//! The lowest protocol level.
//!
//! It operates with PDU's and independent of any transport.

pub mod data_unit;
mod error;
mod exception;
pub mod function;
pub mod r#struct;

pub use self::{error::Error, exception::*};
use crate::protocol::r#struct::{Readable, Writable};

/// Trait that ties function code, arguments and output together.
///
/// Users are free to implement their own functions – be that custom Modbus functions
/// or alternate standard function implementations. In the latter case, consider
/// [making a pull request](https://github.com/eigenein/fennec/pulls).
pub trait Function {
    /// Modbus function code.
    const CODE: u8;

    /// Function arguments type.
    ///
    /// Note that this writable type *must not* include the function code.
    type Args: Writable;

    /// Function result type.
    ///
    /// Note that this readable type *must not* include the function code.
    type Output: Readable;

    /// Convert the payload into PDU.
    fn wrap_args(args: Self::Args) -> data_unit::Request<Self::Args> {
        data_unit::Request { function_code: Self::CODE, args }
    }
}
