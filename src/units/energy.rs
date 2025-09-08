use std::ops::{Div, Mul};

use ordered_float::OrderedFloat;
use rust_decimal::prelude::{ToPrimitive, Zero};
use serde::Deserialize;

use crate::units::{currency::Cost, rate::EuroPerKilowattHour};

#[derive(
    Copy,
    Clone,
    Deserialize,
    Debug,
    derive_more::Display,
    derive_more::FromStr,
    derive_more::Sum,
    derive_more::Add,
    derive_more::Sub,
    derive_more::AddAssign,
    derive_more::SubAssign,
)]
pub struct KilowattHours(pub f64);

impl KilowattHours {
    pub const ZERO: Self = Self(0.0);

    pub const fn max(self, rhs: Self) -> Self {
        Self(self.0.max(rhs.0))
    }

    pub const fn min(self, rhs: Self) -> Self {
        Self(self.0.min(rhs.0))
    }

    pub const fn clamp(self, min: Self, max: Self) -> Self {
        Self(self.0.clamp(min.0, max.0))
    }

    pub fn is_non_positive(self) -> bool {
        self.0.is_sign_negative() || self.0.is_zero()
    }

    pub fn is_non_negative(self) -> bool {
        self.0.is_sign_positive() || self.0.is_zero()
    }
}

impl Mul<f64> for KilowattHours {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Div<f64> for KilowattHours {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Mul<EuroPerKilowattHour> for KilowattHours {
    type Output = Cost;

    fn mul(self, rhs: EuroPerKilowattHour) -> Self::Output {
        Cost(OrderedFloat(self.0 * rhs.0.to_f64().unwrap())) // FIXME: `unwrap`.
    }
}
