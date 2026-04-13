//! Shared structures for reading multiple registers.

use alloc::{boxed::Box, vec::Vec};
use core::fmt::Debug;

use binrw::{BinRead, BinWrite, binwrite};
use bon::bon;

use crate::protocol;

/// Arguments to read a contiguous block of registers.
///
/// # Example
///
/// ```rust
/// use fennec_modbus::protocol::{function::read_registers::Args, r#struct::Writable};
///
/// let bytes = Args::builder().starting_address(107).n_registers(3).build()?.to_bytes()?;
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
#[derive(Copy, Clone, Debug, BinWrite)]
#[bw(big)]
pub struct Args {
    starting_address: u16,
    n_registers: u16,
}

#[bon]
impl Args {
    #[builder]
    pub fn new(
        /// *Zero-based* address of the first register to read.
        starting_address: u16,
        /// Number of registers to read.
        n_registers: u16,
    ) -> Result<Self, protocol::Error> {
        if (1..=125).contains(&n_registers) {
            Ok(Self { starting_address, n_registers })
        } else {
            Err(protocol::Error::InvalidCount(n_registers.into()))
        }
    }
}

/// Arguments to read the number of registers known at compile time.
#[must_use]
#[binwrite]
#[derive(Copy, Clone, Debug)]
#[bw(big)]
pub struct ArgsExact<const N: usize> {
    starting_address: u16,

    #[bw(try_calc = u16::try_from(N))]
    n_registers: u16,
}

impl<const N: usize> ArgsExact<N> {
    pub const fn new(starting_address: u16) -> Self {
        Self { starting_address }
    }
}

/// Output of a contiguous block of registers.
///
/// # Example
///
/// ```rust
/// use fennec_modbus::protocol::{function::read_registers::Output, r#struct::Readable};
///
/// let output = Output::from_bytes(&[
///     0x06, // byte count
///     0x02, 0x2B, // value: high, low
///     0x00, 0x00, // value: high, low
///     0x00, 0x64, // value: high, low
/// ])?;
/// assert_eq!(output.words, [555, 0, 100]);
/// # Ok::<_, anyhow::Error>(())
/// ```
#[must_use]
#[derive(Clone, derive_more::Debug, BinRead)]
#[br(big)]
pub struct Output {
    pub n_bytes: u8,

    #[br(assert(n_bytes.is_multiple_of(2)), count = n_bytes / 2)]
    pub words: Vec<u16>,
}

/// Output of a contiguous block of registers of a compile-time known size.
#[must_use]
#[derive(Clone, derive_more::Debug, BinRead)]
#[br(big)]
pub struct OutputExact<const N: usize> {
    #[br(assert(usize::from(n_bytes) == 2 * N))]
    pub n_bytes: u8,

    pub words: [u16; N],
}
