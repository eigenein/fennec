use std::ops::{Div, Mul};

use ordered_float::OrderedFloat;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    derive_more::Add,
    derive_more::AddAssign,
    derive_more::Display,
    derive_more::From,
    derive_more::FromStr,
    derive_more::Neg,
    derive_more::Sub,
    derive_more::SubAssign,
    derive_more::Sum,
)]
pub struct Quantity<T, const POWER: isize, const AREA: isize, const TIME: isize, const COST: isize>(
    pub T,
);

#[allow(dead_code)]
pub type Bare<T> = Quantity<T, 0, 0, 0, 0>;

impl<T, const POWER: isize, const AREA: isize, const TIME: isize, const COST: isize>
    Quantity<T, POWER, AREA, TIME, COST>
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

impl<const POWER: isize, const AREA: isize, const TIME: isize, const COST: isize>
    Quantity<f64, POWER, AREA, TIME, COST>
{
    pub const ZERO: Self = Self(0.0);
    pub const ONE: Self = Self(1.0);
}

impl<const POWER: isize, const AREA: isize, const TIME: isize, const COST: isize>
    Quantity<OrderedFloat<f64>, POWER, AREA, TIME, COST>
{
    pub const ZERO: Self = Self(OrderedFloat(0.0));
}

impl<const POWER: isize, const AREA: isize, const TIME: isize, const COST: isize>
    Quantity<Decimal, POWER, AREA, TIME, COST>
{
    #[allow(dead_code)]
    pub const ZERO: Self = Self(Decimal::ZERO);

    #[allow(dead_code)]
    pub const ONE: Self = Self(Decimal::ONE);
}

impl<L, R, const POWER: isize, const AREA: isize, const TIME: isize, const COST: isize> Mul<R>
    for Quantity<L, POWER, AREA, TIME, COST>
where
    L: Mul<R>,
{
    type Output = Quantity<L::Output, POWER, AREA, TIME, COST>;

    fn mul(self, rhs: R) -> Self::Output {
        Quantity(self.0 * rhs)
    }
}

impl<L, R, const POWER: isize, const AREA: isize, const TIME: isize, const COST: isize> Div<R>
    for Quantity<L, POWER, AREA, TIME, COST>
where
    L: Div<R>,
{
    type Output = Quantity<L::Output, POWER, AREA, TIME, COST>;

    fn div(self, rhs: R) -> Self::Output {
        Quantity(self.0 / rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
