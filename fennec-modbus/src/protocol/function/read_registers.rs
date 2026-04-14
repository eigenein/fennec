//! Shared structures for reading multiple registers.

mod value;

use alloc::vec::Vec;
use core::{fmt::Debug, marker::PhantomData};

use binrw::{BinRead, BinWrite};

pub use self::value::*;
use crate::{
    protocol,
    protocol::{function, r#struct::Readable},
};

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
#[derive(Copy, Clone, Debug, BinWrite)]
#[bw(big)]
pub struct Args<V: Value> {
    /// *Zero-based* address of the first register to read.
    starting_address: u16,

    /// Number of registers to read.
    n_registers: u16,

    /// Binding to the value type.
    ///
    /// It is not used directly here, but it is useful to ensure correct calculation
    /// for the number of requested registers in the function.
    phantom_data: PhantomData<V>,
}

impl<V: Value> Args<V> {
    /// Number of registers to read.
    #[must_use]
    pub const fn n_registers(&self) -> u16 {
        self.n_registers
    }

    #[expect(clippy::missing_panics_doc)]
    pub fn new(starting_address: u16, n_values: usize) -> Result<Self, protocol::Error> {
        let n_registers = n_values * V::N_BYTES / 2;
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
#[derive(Clone, derive_more::Debug, BinRead)]
#[br(big)]
pub struct Output<V: Value> {
    pub n_bytes: u8,

    #[br(
        assert(usize::from(n_bytes).is_multiple_of(V::N_BYTES)),
        count = usize::from(n_bytes) / V::N_BYTES,
    )]
    pub values: Vec<V>,
}

/// Output of contiguous block of registers with size known at compilation time.
///
/// # Type parameters
///
/// - `N`: number of *values*, one value may span multiple registers
/// - `V`: value type
///
/// # Example
///
/// ```rust
/// use fennec_modbus::protocol::{
///     function::read_registers::{BigEndianI32, OutputExact},
///     r#struct::Readable,
/// };
///
/// let output = OutputExact::<1, BigEndianI32>::from_bytes(&[
///     0x04, // byte count
///     0x00, 0x00, // high word: high byte, low byte
///     0x00, 0x01, // low word: high byte, low byte
/// ])?;
/// assert_eq!(i32::from(output.values[0]), 1);
/// # Ok::<_, anyhow::Error>(())
/// ```
#[must_use]
#[derive(Clone, derive_more::Debug, BinRead)]
#[br(big)]
pub struct OutputExact<const N: usize, V: Value> {
    #[br(assert(usize::from(n_bytes) == V::N_BYTES * N))]
    pub n_bytes: u8,

    pub values: [V; N],
}

/// Value that can be read from contiguous block of registers.
pub trait Value: Readable + 'static {
    /// Number of bytes occupied by a single value.
    const N_BYTES: usize;
}
