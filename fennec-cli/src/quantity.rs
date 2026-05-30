#[macro_use]
pub mod macros;

pub mod currency;
pub mod energy;
mod fmt;
pub mod power;
pub mod price;
pub mod ratios;
pub mod time;
mod zero;

use std::ops::{Div, Mul};

pub use self::{fmt::Format, zero::Zero};

/// Generic quantity with dimensions of `P` over power, `T` over time, and `C` over cost.
///
/// The parameter `M` is the order of magnitude.
#[must_use]
#[repr(transparent)]
#[derive(
    derive_more::Add,
    derive_more::AddAssign,
    derive_more::Constructor,
    derive_more::FromStr,
    derive_more::Neg,
    derive_more::Sub,
    derive_more::SubAssign,
    derive_more::Sum,
    musli::Decode,
    musli::Encode,
    serde::Deserialize,
    serde::Serialize,
    std::clone::Clone,
    std::cmp::Eq,
    std::cmp::Ord,
    std::cmp::PartialEq,
    std::cmp::PartialOrd,
    std::marker::Copy,
)]
#[musli(transparent)]
pub struct Quantity<V, const M: i8, const P: i8, const T: i8, const C: i8>(pub V);

impl<V, const M: i8, const P: i8, const T: i8, const C: i8> Quantity<V, M, P, T, C> {
    pub fn rescale<const TM: i8>(self) -> Quantity<f64, TM, P, T, C>
    where
        V: Into<f64>,
    {
        Quantity(self.0.into() * 10.0_f64.powi(i32::from(M - TM)))
    }
}

impl<const M: i8, const P: i8, const T: i8, const C: i8> Quantity<f64, M, P, T, C> {
    pub const fn min(self, rhs: Self) -> Self {
        Self(self.0.min(rhs.0))
    }

    pub const fn max(self, rhs: Self) -> Self {
        Self(self.0.max(rhs.0))
    }
}

impl<V, const M: i8, const P: i8, const T: i8, const C: i8> Mul<V> for Quantity<V, M, P, T, C>
where
    V: Mul<Output = V>,
{
    type Output = Self;

    /// Multiply the quantity by a bare scalar.
    fn mul(self, rhs: V) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl<const M: i8, const P: i8, const T: i8, const C: i8> Mul<Quantity<Self, M, P, T, C>> for f64 {
    type Output = Quantity<Self, M, P, T, C>;

    /// Multiply the bare scalar by a quantity.
    fn mul(self, rhs: Quantity<Self, M, P, T, C>) -> Self::Output {
        Quantity(self * rhs.0)
    }
}

impl<V: Div, const M: i8, const P: i8, const T: i8, const C: i8> Div<Self>
    for Quantity<V, M, P, T, C>
{
    type Output = <V as Div>::Output;

    /// Divide the quantity by the same kind of quantity producing a bare scalar.
    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl<V: Div<Output = V>, const M: i8, const P: i8, const T: i8, const C: i8> Div<V>
    for Quantity<V, M, P, T, C>
{
    type Output = Self;

    /// Divide the quantity by a scalar.
    fn div(self, rhs: V) -> Self::Output {
        Self(self.0 / rhs)
    }
}

#[cfg(test)]
mod tests {
    use crate::quantity::energy::{KilowattHours, WattHours};

    /// Verify that I haven't screwed up the magnitude arithmetic. 😃
    #[test]
    fn rescale() {
        assert_eq!(WattHours::new(10.0_f64).rescale(), KilowattHours::new(0.01_f64));
    }
}
