use std::{
    fmt::{Debug, Display, Formatter},
    ops::{Div, Mul},
};

use chrono::TimeDelta;
use ordered_float::OrderedFloat;

use crate::quantity::{Quantity, cost::Cost, power::Kilowatts, rate::KilowattHourRate};

pub type KilowattHours = Quantity<1, 1, 0>;

impl KilowattHours {
    /// 1 Wh.
    pub const ONE_THOUSANDTH: Self = Self(OrderedFloat(0.001));

    pub const fn zero() -> Self {
        Self::ZERO
    }

    #[must_use]
    pub const fn is_significant(self) -> bool {
        self.0.0 >= Self::ONE_THOUSANDTH.0.0
    }

    pub fn from_watt_hours_u32(watt_hours: u32) -> Self {
        Self::from(f64::from(watt_hours) * 0.001)
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

        #[expect(clippy::cast_possible_truncation)]
        TimeDelta::seconds((hours.0 * 3600.0) as i64)
    }
}

impl Div<TimeDelta> for KilowattHours {
    type Output = Kilowatts;

    fn div(self, rhs: TimeDelta) -> Self::Output {
        let hours = rhs.as_seconds_f64() / 3600.0;
        Quantity(self.0 / hours)
    }
}
