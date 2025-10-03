pub mod currency;
pub mod energy;
pub mod power;
pub mod rate;

use std::ops::{Div, Mul};

use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Deserialize,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    derive_more::Add,
    derive_more::AddAssign,
    derive_more::From,
    derive_more::FromStr,
    derive_more::Neg,
    derive_more::Sub,
    derive_more::SubAssign,
    derive_more::Sum,
)]
pub struct Quantity<T, const POWER: isize, const TIME: isize, const COST: isize>(pub T);

impl<T, const POWER: isize, const TIME: isize, const COST: isize> Quantity<T, POWER, TIME, COST>
where
    Self: PartialOrd,
{
    pub fn min(mut self, rhs: Self) -> Self {
        if rhs < self {
            self = rhs;
        }
        self
    }

    pub fn max(mut self, rhs: Self) -> Self {
        if rhs > self {
            self = rhs;
        }
        self
    }

    pub fn clamp(mut self, min: Self, max: Self) -> Self {
        if self < min {
            self = min;
        }
        if self > max {
            self = max;
        }
        self
    }
}

impl<const POWER: isize, const TIME: isize, const COST: isize> Quantity<f64, POWER, TIME, COST> {
    pub const ZERO: Self = Self(0.0);
}

impl<T, const POWER: isize, const TIME: isize, const COST: isize> Mul<T>
    for Quantity<T, POWER, TIME, COST>
where
    T: Mul<T>,
{
    type Output = Quantity<T::Output, POWER, TIME, COST>;

    fn mul(self, rhs: T) -> Self::Output {
        Quantity(self.0 * rhs)
    }
}

impl<T, const POWER: isize, const TIME: isize, const COST: isize> Div<T>
    for Quantity<T, POWER, TIME, COST>
where
    T: Div<T>,
{
    type Output = Quantity<T::Output, POWER, TIME, COST>;

    fn div(self, rhs: T) -> Self::Output {
        Quantity(self.0 / rhs)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::{Debug, Formatter};

    use super::*;

    pub type Bare<T> = Quantity<T, 0, 0, 0>;

    impl<T: Debug> Debug for Bare<T> {
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
