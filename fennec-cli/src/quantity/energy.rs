use std::ops::{Div, Mul};

use crate::quantity::{
    cost::Cost,
    power::Kilowatts,
    proportions::{BasisPoints, Percentage},
    rate::KilowattHourRate,
    time::Hours,
};

quantity!(MilliwattHours, via: i64, suffix: "mWh", precision: 0);
quantity!(DecawattHours, via: u16, suffix: "daWh", precision: 1);
quantity!(KilowattHours, via: f64, suffix: "kWh", precision: 3);

mul!(Kilowatts, Hours, KilowattHours);

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

impl Div<Hours> for KilowattHours {
    type Output = Kilowatts;

    fn div(self, hours: Hours) -> Self::Output {
        Kilowatts(self.0 / hours.0)
    }
}
