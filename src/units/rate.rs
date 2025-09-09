use std::ops::{Div, Mul};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    derive_more::Display,
    derive_more::FromStr,
    derive_more::Sub,
    derive_more::Add,
    derive_more::Neg,
    derive_more::Sum,
    Serialize,
    Deserialize,
)]
pub struct KilowattHourRate(#[serde_as(as = "serde_with::DisplayFromStr")] pub Decimal);

impl Mul<Decimal> for KilowattHourRate {
    type Output = Self;

    fn mul(self, rhs: Decimal) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Div<usize> for KilowattHourRate {
    type Output = Self;

    fn div(self, rhs: usize) -> Self::Output {
        Self(self.0 / Decimal::from(rhs))
    }
}
