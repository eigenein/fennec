mod kilowatt_hours;

use std::ops::Mul;

pub use self::kilowatt_hours::KilowattHours;
use crate::quantity::proportions::BasisPoints;

quantity!(MilliwattHours, i64, "mWh");
quantity!(DecawattHours, u16, "daWh");

impl From<DecawattHours> for KilowattHours {
    fn from(value: DecawattHours) -> Self {
        Self(0.01 * f64::from(value.0))
    }
}

impl Mul<BasisPoints> for DecawattHours {
    type Output = MilliwattHours;

    fn mul(self, rhs: BasisPoints) -> Self::Output {
        MilliwattHours(i64::from(self.0) * i64::from(rhs.0))
    }
}
