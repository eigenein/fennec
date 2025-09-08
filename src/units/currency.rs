use std::ops::Mul;

use ordered_float::OrderedFloat;

#[derive(
    derive_more::Add,
    Ord,
    PartialOrd,
    derive_more::Display,
    PartialEq,
    Eq,
    Copy,
    Clone,
    derive_more::AddAssign,
    derive_more::SubAssign,
    derive_more::Sub,
)]
pub struct Cost(pub OrderedFloat<f64>);

impl Cost {
    pub const ZERO: Self = Self(OrderedFloat(0.0));
}

impl From<Cost> for f64 {
    fn from(cost: Cost) -> Self {
        cost.0.0
    }
}

impl Mul<f64> for Cost {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}
