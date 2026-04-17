pub trait BitSize {
    /// Number of bits occupied by the value.
    const N_BITS: usize;

    /// Number of whole bytes occupied by the value.
    const N_BYTES: usize = Self::N_BITS.div_ceil(8);
}

macro_rules! impl_for {
    ($type:ty, $n_bits:literal) => {
        impl BitSize for $type {
            const N_BITS: usize = $n_bits;
        }
    };
}

impl_for!(u16, 16);
impl_for!(i16, 16);
impl_for!(u32, 32);
impl_for!(i32, 32);
impl_for!(u64, 64);
impl_for!(i64, 64);
impl_for!(u128, 128);
impl_for!(i128, 128);
