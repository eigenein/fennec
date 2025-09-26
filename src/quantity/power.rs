use std::ops::Mul;

use crate::quantity::{
    Quantity,
    energy::KilowattHours,
    rate::{HourRate, KilowattHourRate},
    time::Hours,
};

pub type Kilowatts = Quantity<f64, 1, 0, 0, 0>;

impl Kilowatts {
    #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn into_watts_u32(self) -> u32 {
        (self.0 * 1000.0).round() as u32
    }
}

impl Mul<Hours> for Kilowatts {
    type Output = KilowattHours;

    fn mul(self, rhs: Hours) -> Self::Output {
        Quantity(self.0 * rhs.0)
    }
}

impl Mul<KilowattHourRate> for Kilowatts {
    type Output = HourRate;

    fn mul(self, rhs: KilowattHourRate) -> Self::Output {
        Quantity(self.0 * rhs.0)
    }
}
