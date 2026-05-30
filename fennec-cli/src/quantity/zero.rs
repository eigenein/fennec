use crate::quantity::Quantity;

pub trait Zero {
    const ZERO: Self;
}

impl Zero for u8 {
    const ZERO: Self = 0;
}

impl Zero for u16 {
    const ZERO: Self = 0;
}

impl Zero for u32 {
    const ZERO: Self = 0;
}

impl Zero for i64 {
    const ZERO: Self = 0;
}

impl Zero for f64 {
    const ZERO: Self = 0.0;
}

impl<V: Zero, const P: i8, const T: i8, const C: i8> Zero for Quantity<V, P, T, C> {
    const ZERO: Self = Self(V::ZERO);
}
