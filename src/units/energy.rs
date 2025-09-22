use std::ops::{Div, Mul};

use crate::units::{
    currency::Cost,
    power::Kilowatts,
    quantity::Quantity,
    rate::KilowattHourRate,
    time::Hours,
};

pub type KilowattHours = Quantity<f64, 1, 0, 1, 0>;

impl KilowattHours {
    pub fn from_watt_hours_u32(watt_hours: u32) -> Self {
        Self(watt_hours as f64 * 0.001)
    }
}

impl Mul<KilowattHourRate> for KilowattHours {
    type Output = Cost;

    fn mul(self, rhs: KilowattHourRate) -> Self::Output {
        Cost::from(self.0 * rhs.0)
    }
}

impl Div<Kilowatts> for KilowattHours {
    type Output = Hours;

    fn div(self, rhs: Kilowatts) -> Self::Output {
        Quantity(self.0 / rhs.0)
    }
}

impl Div<Hours> for KilowattHours {
    type Output = Kilowatts;

    fn div(self, rhs: Hours) -> Self::Output {
        Quantity(self.0 / rhs.0)
    }
}
