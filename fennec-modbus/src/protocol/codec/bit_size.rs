pub trait BitSize {
    /// Number of bits occupied by the value.
    const N_BITS: u16;

    /// Number of whole bytes occupied by the value.
    const N_BYTES: u16 = Self::N_BITS.div_ceil(8);

    /// Number of whole words occupied by the value.
    const N_WORDS: u16 = Self::N_BITS.div_ceil(16);
}

macro_rules! impl_for {
    ($type:ty, $n_bits:literal) => {
        impl BitSize for $type {
            const N_BITS: u16 = $n_bits;
        }
    };
}

impl_for!(u8, 8);
impl_for!(i8, 8);
impl_for!(u16, 16);
impl_for!(i16, 16);
impl_for!(u32, 32);
impl_for!(i32, 32);
impl_for!(u64, 64);
impl_for!(i64, 64);
impl_for!(u128, 128);
impl_for!(i128, 128);
