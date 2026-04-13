//! Implementations of composite types that span multiple registers.

use binrw::BinRead;

use crate::protocol::function::read_registers::Value;

/// Trivial implementation for `u16`, it's just read natively from Modbus in big-endian.
impl Value for u16 {
    const N_BYTES: usize = 2;
}

/// Trivial implementation for `i16`, it's just read natively from Modbus in big-endian.
impl Value for i16 {
    const N_BYTES: usize = 2;
}

/// [`i32`] composed of two words in big-endian ordering.
///
/// It's a trivial implementation since two big-endian words in big-endian is just [`i32`] in big-endian.
///
/// # Example
///
/// ```rust
/// use fennec_modbus::protocol::{function::read_registers::BigEndianI32, r#struct::Readable};
/// assert_eq!(i32::from(BigEndianI32::from_bytes(&[0xFF, 0xFF, 0xFF, 0xFE])?), -2);
/// # Ok::<_, anyhow::Error>(())
/// ```
#[derive(Copy, Clone, BinRead, derive_more::Into)]
#[br(big)]
pub struct BigEndianI32(i32);

impl Value for BigEndianI32 {
    const N_BYTES: usize = 4;
}

/// [`u32`] composed of two words in big-endian ordering.
///
/// It's a trivial implementation since two big-endian words in big-endian is just [`u32`] in big-endian.
#[derive(Copy, Clone, BinRead, derive_more::Into)]
#[br(big)]
pub struct BigEndianU32(u32);

impl Value for BigEndianU32 {
    const N_BYTES: usize = 4;
}
