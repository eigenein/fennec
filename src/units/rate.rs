use std::ops::{Div, Mul};

use crate::units::{Cost, Hours, Kilowatts, Quantity};

/// Euro per kilowatt-hour.
pub type KilowattHourRate = Quantity<f64, 1, 0, 1, -1>;

/// Euro per hour.
pub type HourRate = Quantity<f64, 0, 0, -1, 1>;

impl Mul<Hours> for HourRate {
    type Output = Cost;

    fn mul(self, rhs: Hours) -> Self::Output {
        Quantity(self.0 * rhs.0)
    }
}

impl Div<Kilowatts> for HourRate {
    type Output = KilowattHourRate;

    fn div(self, rhs: Kilowatts) -> Self::Output {
        Quantity(self.0 / rhs.0)
    }
}
