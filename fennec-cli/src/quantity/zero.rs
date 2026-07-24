//! Zero implementation for different underlying types.

use chrono::TimeDelta;

use crate::quantity::Quantity;

pub trait Zero {
    const ZERO: Self;
}

/// [`Zero`] for [`usize`].
impl<const M: i8, const P: i8, const T: i8, const C: i8> Zero for Quantity<usize, M, P, T, C> {
    const ZERO: Self = Self(0);
}

/// [`Zero`] for [`u8`].
impl<const M: i8, const P: i8, const T: i8, const C: i8> Zero for Quantity<u8, M, P, T, C> {
    const ZERO: Self = Self(0);
}

/// [`Zero`] for [`u32`].
impl<const M: i8, const P: i8, const T: i8, const C: i8> Zero for Quantity<u32, M, P, T, C> {
    const ZERO: Self = Self(0);
}

/// [`Zero`] for [`i64`].
impl<const M: i8, const P: i8, const T: i8, const C: i8> Zero for Quantity<i64, M, P, T, C> {
    const ZERO: Self = Self(0);
}

/// [`Zero`] for [`f64`].
impl<const M: i8, const P: i8, const T: i8, const C: i8> Zero for Quantity<f64, M, P, T, C> {
    const ZERO: Self = Self(0.0);
}

impl Zero for TimeDelta {
    const ZERO: Self = Self::zero();
}
