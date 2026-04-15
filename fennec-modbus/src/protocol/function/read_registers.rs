//! Shared structures for reading multiple registers.

use alloc::vec::Vec;
use core::{fmt::Debug, marker::PhantomData};

use deku::{DekuContainerRead, DekuRead, DekuSize, DekuWrite};

use crate::{protocol, protocol::function};

/// Read holding registers.
pub struct Holding;

impl function::Code for Holding {
    const CODE: u8 = 3;
}

/// Read input registers.
pub struct Input;

impl function::Code for Input {
    const CODE: u8 = 4;
}

/// Arguments to read a contiguous block of registers.
///
/// # Example
///
/// ```rust
/// use fennec_modbus::protocol::{function::read_registers::Args, r#struct::Writable};
///
/// let args = Args::<u16>::new(107, 3)?;
/// assert_eq!(args.n_registers(), 3);
///
/// let bytes = args.to_bytes()?;
/// assert_eq!(
///     bytes,
///     [
///         0x00, 0x6B, // starting address: high, low
///         0x00, 0x03, // count: high, low
///     ]
/// );
/// # Ok::<_, anyhow::Error>(())
/// ```
#[must_use]
#[derive(Copy, Clone, Debug, DekuWrite)]
#[deku(endian = "big")]
pub struct Args<V> {
    /// *Zero-based* address of the first register to read.
    starting_address: u16,

    /// Number of registers to read.
    n_registers: u16,

    /// Binding to the value type.
    ///
    /// It is not used directly here, but it is useful to ensure correct calculation
    /// for the number of requested registers in the function.
    #[deku(skip)]
    phantom_data: PhantomData<V>,
}

impl<V> Args<V> {
    /// Number of registers to read.
    #[must_use]
    pub const fn n_registers(&self) -> u16 {
        self.n_registers
    }

    #[expect(clippy::missing_panics_doc)]
    pub fn new(starting_address: u16, n_values: usize) -> Result<Self, protocol::Error>
    where
        V: DekuSize,
    {
        let n_registers = n_values * V::SIZE_BYTES.unwrap() / 2;
        if (1..=125).contains(&n_registers) {
            Ok(Self {
                starting_address,
                n_registers: u16::try_from(n_registers).unwrap(),
                phantom_data: PhantomData,
            })
        } else {
            Err(protocol::Error::InvalidCount(n_registers))
        }
    }
}

/// Output of a contiguous block of registers.
///
/// # Type parameters
///
/// - `V`: value type, one value may span multiple registers
///
/// # Example
///
/// ```rust
/// use fennec_modbus::protocol::{function::read_registers::Output, r#struct::Readable};
///
/// let output = Output::<u16>::from_bytes(&[
///     0x06, // byte count
///     0x02, 0x2B, // value: high, low
///     0x00, 0x00, // value: high, low
///     0x00, 0x64, // value: high, low
/// ])?;
/// assert_eq!(output.values, [555, 0, 100]);
/// # Ok::<_, anyhow::Error>(())
/// ```
#[must_use]
#[derive(Clone, derive_more::Debug, DekuRead)]
pub struct Output<V: for<'a> DekuContainerRead<'a>> {
    /// FIXME: assert number of bytes?
    pub n_bytes: u8,

    #[deku(bytes_read = "n_bytes")]
    pub values: Vec<V>,
}
