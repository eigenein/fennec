use std::ops::Mul;

use ordered_float::OrderedFloat;
use rust_decimal::prelude::ToPrimitive;

use crate::units::{Quantity, currency::Cost, rate::KilowattHourRate};

pub type KilowattHours = Quantity<f64, 1, 0, 1, 0>;

impl Mul<KilowattHourRate> for KilowattHours {
    type Output = Cost;

    fn mul(self, rhs: KilowattHourRate) -> Self::Output {
        Cost::from(OrderedFloat(self.0 * rhs.0.to_f64().unwrap())) // FIXME: `unwrap`.
    }
}
