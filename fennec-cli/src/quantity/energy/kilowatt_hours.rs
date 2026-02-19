use std::ops::{Div, Mul};

use chrono::TimeDelta;

use crate::quantity::{
    Quantity,
    cost::Cost,
    energy::MilliwattHours,
    power::Kilowatts,
    proportions::Percentage,
    rate::KilowattHourRate,
};

quantity!(KilowattHours, f64, "kWh");

impl KilowattHours {
    pub const ONE_WATT_HOUR: Self = Self(0.001);
}

impl From<MilliwattHours> for KilowattHours {
    fn from(value: MilliwattHours) -> Self {
        #[expect(clippy::cast_precision_loss)]
        Self(value.0 as f64 * 0.000_001)
    }
}

impl Mul<Percentage> for KilowattHours {
    type Output = Self;

    fn mul(self, percentage: Percentage) -> Self::Output {
        self * percentage.to_proportion()
    }
}

impl Mul<KilowattHourRate> for KilowattHours {
    type Output = Cost;

    fn mul(self, rhs: KilowattHourRate) -> Self::Output {
        Quantity(self.0 * rhs.0)
    }
}

impl Div<TimeDelta> for KilowattHours {
    type Output = Kilowatts;

    fn div(self, rhs: TimeDelta) -> Self::Output {
        let hours = rhs.as_seconds_f64() / 3600.0;
        assert!(hours.is_finite());
        Quantity(self.0 / hours)
    }
}
