use std::ops::Mul;

use chrono::TimeDelta;

use crate::quantity::energy::KilowattHours;

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

impl Mul<TimeDelta> for Kilowatts {
    type Output = KilowattHours;

    fn mul(self, rhs: TimeDelta) -> Self::Output {
        let hours = rhs.as_seconds_f64() / 3600.0;
        KilowattHours(self.0 * hours)
    }
}

impl Mul<TimeDelta> for Watts {
    type Output = KilowattHours;

    fn mul(self, time_delta: TimeDelta) -> Self::Output {
        Kilowatts::from(self) * time_delta
    }
}
