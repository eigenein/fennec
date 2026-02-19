use std::ops::{Div, Mul};

use chrono::TimeDelta;

use crate::quantity::{
    cost::Cost,
    power::Kilowatts,
    proportions::{BasisPoints, Percentage},
    rate::KilowattHourRate,
};

quantity!(MilliwattHours, i64, "mWh");
quantity!(DecawattHours, u16, "daWh");
quantity!(KilowattHours, f64, "kWh");

impl Mul<BasisPoints> for DecawattHours {
    type Output = MilliwattHours;

    fn mul(self, rhs: BasisPoints) -> Self::Output {
        MilliwattHours(i64::from(self.0) * i64::from(rhs.0))
    }
}

impl From<DecawattHours> for KilowattHours {
    fn from(value: DecawattHours) -> Self {
        Self(0.01 * f64::from(value.0))
    }
}

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
        Cost(self.0 * rhs.0)
    }
}

impl Div<TimeDelta> for KilowattHours {
    type Output = Kilowatts;

    fn div(self, rhs: TimeDelta) -> Self::Output {
        let hours = rhs.as_seconds_f64() / 3600.0;
        assert!(hours.is_finite());
        Kilowatts(self.0 / hours)
    }
}
