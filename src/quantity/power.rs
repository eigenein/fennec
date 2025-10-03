use std::{
    fmt::{Display, Formatter},
    ops::Mul,
};

use chrono::TimeDelta;

use crate::quantity::{Quantity, energy::KilowattHours};

pub type Kilowatts = Quantity<f64, 1, 0, 0>;

impl Kilowatts {
    pub fn from_watts(watts: f64) -> Self {
        Self(watts / 1000.0)
    }

    #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn into_watts_u32(self) -> u32 {
        (self.0 * 1000.0).round() as u32
    }
}

impl Display for Kilowatts {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} kW", self.0)
    }
}

impl Mul<TimeDelta> for Kilowatts {
    type Output = KilowattHours;

    fn mul(self, rhs: TimeDelta) -> Self::Output {
        let hours = rhs.as_seconds_f64() / 3600.0;
        Quantity(self.0 * hours)
    }
}
