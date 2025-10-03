use std::{
    fmt::{Display, Formatter},
    ops::Mul,
};

use chrono::TimeDelta;

use crate::quantity::{Quantity, energy::KilowattHours};

pub type Kilowatts = Quantity<f64, 1, 0, 0>;

impl Display for Kilowatts {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} kW", self.0)
    }
}

impl From<Kilowatts> for opentelemetry::Value {
    fn from(value: Kilowatts) -> Self {
        format!("{:.0}W", value.0 * 1000.0).into()
    }
}

impl Mul<TimeDelta> for Kilowatts {
    type Output = KilowattHours;

    fn mul(self, rhs: TimeDelta) -> Self::Output {
        let hours = rhs.as_seconds_f64() / 3600.0;
        Quantity(self.0 * hours)
    }
}

#[derive(
    Copy, Clone, Eq, PartialEq, derive_more::FromStr, serde::Serialize, serde::Deserialize,
)]
pub struct Watts(pub u32);

impl From<Kilowatts> for Watts {
    #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    fn from(kilowatts: Kilowatts) -> Self {
        Self((kilowatts.0 * 1000.0).round() as u32)
    }
}

impl Display for Watts {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} W", self.0)
    }
}
