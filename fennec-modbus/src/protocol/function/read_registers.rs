//! Shared structures for reading multiple registers.

use core::{fmt::Debug, marker::PhantomData};

use bytes::{Buf, BufMut};

use crate::{
    protocol,
    protocol::{
        BitSize,
        Decode,
        Encode,
        adapters::DropRemaining,
        function,
        function::ArgumentError,
    },
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
/// use fennec_modbus::protocol::{Encode, function::read_registers::Args};
///
/// let args = Args::<u16>::new(107, 3)?;
/// assert_eq!(args.n_registers(), 3);
///
/// let bytes = args.encode_into_bytes();
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
#[derive(Copy, Clone, Debug)]
pub struct Args<V> {
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

impl<V> Args<V> {
    /// Number of registers to read.
    #[must_use]
    pub const fn n_registers(&self) -> u16 {
        self.n_registers
    }

    pub fn new(starting_address: u16, n_values: usize) -> Result<Self, ArgumentError>
    where
        V: BitSize,
    {
        let n_registers = n_values * V::N_BYTES / 2;
        if (1..=125).contains(&n_registers) {
            Ok(Self {
                starting_address,
                n_registers: u16::try_from(n_registers)?,
                phantom_data: PhantomData,
            })
        } else {
            Err(ArgumentError::InvalidRegisterCount(n_registers))
        }
    }
}

impl<V> Encode for Args<V> {
    fn encode_into(&self, buf: &mut impl BufMut) {
        buf.put_u16(self.starting_address);
        buf.put_u16(self.n_registers);
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
/// use fennec_modbus::protocol::{Decode, function::read_registers::Output};
///
/// let mut buf: &[u8] = &[
///     0x06_u8, // byte count
///     0x02, 0x2B, // value: high, low
///     0x00, 0x00, // value: high, low
///     0x00, 0x64, // value: high, low
/// ];
/// let output = Output::<Vec<u16>>::decode_from(&mut buf)?;
/// assert_eq!(output, [555, 0, 100]);
/// # Ok::<_, anyhow::Error>(())
/// ```
#[must_use]
#[derive(Copy, Clone)]
pub struct Output<C>(pub C);

// TODO: extract into `adapters`.
impl<V: Decode> Decode for Output<V> {
    type Output = V::Output;

    fn decode_from(buf: &mut impl Buf) -> Result<Self::Output, protocol::Error> {
        let n_bytes = buf.try_get_u8()?;
        V::decode_from(&mut DropRemaining(buf).take(n_bytes.into()))
    }
}
