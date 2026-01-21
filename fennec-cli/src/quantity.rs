pub mod cost;
pub mod energy;
pub mod power;
pub mod rate;

use std::{
    cmp::Ordering,
    ops::{Div, Mul},
};

use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Deserialize,
    PartialEq,
    PartialOrd,
    Serialize,
    derive_more::Add,
    derive_more::AddAssign,
    derive_more::From,
    derive_more::FromStr,
    derive_more::Into,
    derive_more::Neg,
    derive_more::Sub,
    derive_more::SubAssign,
    derive_more::Sum,
)]
#[from(i32, f64)]
#[into(f64)]
#[must_use]
pub struct Quantity<const POWER: isize, const TIME: isize, const COST: isize>(pub f64);

impl<const POWER: isize, const TIME: isize, const COST: isize> Eq for Quantity<POWER, TIME, COST> {}

impl<const POWER: isize, const TIME: isize, const COST: isize> Ord for Quantity<POWER, TIME, COST> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<const POWER: isize, const TIME: isize, const COST: isize> Quantity<POWER, TIME, COST> {
    pub const ZERO: Self = Self(0.0);

    pub const fn abs(mut self) -> Self {
        self.0 = self.0.abs();
        self
    }
}

impl<const POWER: isize, const TIME: isize, const COST: isize> Mul<f64>
    for Quantity<POWER, TIME, COST>
{
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl<const POWER: isize, const TIME: isize, const COST: isize> Div<f64>
    for Quantity<POWER, TIME, COST>
{
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl<const POWER: isize, const TIME: isize, const COST: isize> Div<Self>
    for Quantity<POWER, TIME, COST>
{
    type Output = f64;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::{Debug, Formatter};

    use super::*;

    pub type Bare = Quantity<0, 0, 0>;

    impl Debug for Bare {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }

    #[test]
    fn test_min() {
        assert_eq!(Bare::from(1).min(Bare::from(2)), Bare::from(1));
        assert_eq!(Bare::from(2).min(Bare::from(1)), Bare::from(1));
    }

    #[test]
    fn test_max() {
        assert_eq!(Bare::from(1).max(Bare::from(2)), Bare::from(2));
        assert_eq!(Bare::from(2).max(Bare::from(1)), Bare::from(2));
    }

    #[test]
    fn test_clamp() {
        assert_eq!(Bare::from(1).clamp(Bare::from(2), Bare::from(3)), Bare::from(2));
        assert_eq!(Bare::from(4).clamp(Bare::from(2), Bare::from(3)), Bare::from(3));
        assert_eq!(Bare::from(2).clamp(Bare::from(1), Bare::from(3)), Bare::from(2));
    }
}
