use std::ops::Mul;

use crate::quantity::{
    energy::{KilowattHours, MilliwattHours},
    proportions::BasisPoints,
};

/// Decawatt-hours, 1 daWh = 10 Wh.
#[derive(Copy, Clone, derive_more::From)]
pub struct DecawattHours(u16);

impl From<DecawattHours> for KilowattHours {
    fn from(value: DecawattHours) -> Self {
        Self(0.01 * f64::from(value.0))
    }
}

impl Mul<BasisPoints> for DecawattHours {
    type Output = MilliwattHours;

    fn mul(self, rhs: BasisPoints) -> Self::Output {
        MilliwattHours::from(i64::from(self.0) * i64::from(rhs))
    }
}
