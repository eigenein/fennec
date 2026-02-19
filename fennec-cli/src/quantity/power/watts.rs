use std::ops::{Div, Mul};

use chrono::TimeDelta;

use crate::quantity::{energy::KilowattHours, power::kilowatts::Kilowatts};

quantity!(Watts, f64, "W");

impl From<Kilowatts> for Watts {
    fn from(kilowatts: Kilowatts) -> Self {
        Self(kilowatts.0 * 1000.0)
    }
}

impl Mul<TimeDelta> for Watts {
    type Output = KilowattHours;

    fn mul(self, time_delta: TimeDelta) -> Self::Output {
        Kilowatts::from(self) * time_delta
    }
}

impl Div<f64> for Watts {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self(self.0 / rhs)
    }
}
