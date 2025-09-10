use std::ops::Mul;

use ordered_float::OrderedFloat;
use rust_decimal::prelude::{ToPrimitive, Zero};

use crate::units::{Quantity, currency::Cost, rate::KilowattHourRate};

pub type KilowattHours = Quantity<f64, 1, 0, 1, 0>;

impl KilowattHours {
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

impl Mul<KilowattHourRate> for KilowattHours {
    type Output = Cost;

    fn mul(self, rhs: KilowattHourRate) -> Self::Output {
        Cost::new(OrderedFloat(self.0 * rhs.0.to_f64().unwrap())) // FIXME: `unwrap`.
    }
}
