use std::ops::Mul;

use crate::quantity::{energy::KilowattHours, time::Hours};

quantity!(Watts, via: f64, suffix: "W", precision: 0);
quantity!(Kilowatts, via: f64, suffix: "kW", precision: 3);

impl From<Kilowatts> for Watts {
    fn from(kilowatts: Kilowatts) -> Self {
        Self(kilowatts.0 * 1000.0)
    }
}

impl From<Watts> for Kilowatts {
    fn from(watts: Watts) -> Self {
        Self(watts.0 / 1000.0)
    }
}

impl Mul<Hours> for Watts {
    type Output = KilowattHours;

    fn mul(self, hours: Hours) -> Self::Output {
        Kilowatts::from(self) * hours
    }
}
