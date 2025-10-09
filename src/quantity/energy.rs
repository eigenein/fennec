use std::{
    fmt::{Debug, Display, Formatter},
    ops::{Div, Mul},
};

use chrono::TimeDelta;

use crate::quantity::{Quantity, cost::Cost, power::Kilowatts, rate::KilowattHourRate};

pub type KilowattHours = Quantity<f64, 1, 1, 0>;

impl KilowattHours {
    pub fn from_watt_hours_u32(watt_hours: u32) -> Self {
        Self(f64::from(watt_hours) * 0.001)
    }

    /// FIXME: move to [`Quantity`].
    pub const fn abs(mut self) -> Self {
        self.0 = self.0.abs();
        self
    }
}

impl Default for KilowattHours {
    fn default() -> Self {
        Self(0.0)
    }
}

impl Display for KilowattHours {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.0} Wh", self.0 * 1000.0)
    }
}

impl Debug for KilowattHours {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.0}Wh", self.0 * 1000.0)
    }
}

impl From<KilowattHours> for opentelemetry::Value {
    fn from(value: KilowattHours) -> Self {
        format!("{value:?}").into()
    }
}

impl Mul<KilowattHourRate> for KilowattHours {
    type Output = Cost;

    fn mul(self, rhs: KilowattHourRate) -> Self::Output {
        Cost::from(self.0 * rhs.0)
    }
}

impl Div<Kilowatts> for KilowattHours {
    type Output = TimeDelta;

    fn div(self, rhs: Kilowatts) -> Self::Output {
        let hours = self.0 / rhs.0;

        #[allow(clippy::cast_possible_truncation)]
        TimeDelta::seconds((hours * 3600.0) as i64)
    }
}

impl Div<TimeDelta> for KilowattHours {
    type Output = Kilowatts;

    fn div(self, rhs: TimeDelta) -> Self::Output {
        let hours = rhs.as_seconds_f64() / 3600.0;
        Quantity(self.0 / hours)
    }
}
