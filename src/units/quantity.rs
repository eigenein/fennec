use std::ops::{Div, Mul};

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
    derive_more::Constructor,
    derive_more::Display,
    derive_more::FromStr,
    derive_more::Neg,
    derive_more::Sub,
    derive_more::SubAssign,
    derive_more::Sum,
)]
pub struct Quantity<T, const POWER: isize, const AREA: isize, const TIME: isize, const COST: isize>(
    /// FIXME: eventually make private.
    pub T,
);

impl<const POWER: isize, const AREA: isize, const TIME: isize, const COST: isize>
    Quantity<f64, POWER, AREA, TIME, COST>
{
    pub const ZERO: Self = Self(0.0);
    pub const ONE: Self = Self(1.0);
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
