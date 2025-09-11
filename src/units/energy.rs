use std::ops::Mul;

use crate::units::{Quantity, currency::Cost, rate::KilowattHourRate};

pub type KilowattHours = Quantity<f64, 1, 0, 1, 0>;

impl Mul<KilowattHourRate> for KilowattHours {
    type Output = Cost;

    fn mul(self, rhs: KilowattHourRate) -> Self::Output {
        Cost::from(self.0 * rhs.0)
    }
}
