use std::ops::{Div, Mul};

use crate::quantity::{
    power::Watts,
    proportions::{BasisPoints, Percentage},
    time::Hours,
};

quantity!(MilliwattHours, via: i64, suffix: "mWh", precision: 0);
quantity!(WattHours, via: f64, suffix: "Wh", precision: 0);
quantity!(DecawattHours, via: u16, suffix: "daWh", precision: 1);
quantity!(KilowattHours, via: f64, suffix: "kWh", precision: 3);

mul!(Watts, Hours, WattHours);

impl WattHours {
    pub const ONE: Self = Self(1.0);
}

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

impl From<DecawattHours> for WattHours {
    fn from(value: DecawattHours) -> Self {
        Self(f64::from(value.0) * 10.0)
    }
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

impl Mul<Percentage> for WattHours {
    type Output = Self;

    fn mul(self, percentage: Percentage) -> Self::Output {
        self * percentage.to_proportion()
    }
}

impl Div<Hours> for WattHours {
    type Output = Watts;

    fn div(self, hours: Hours) -> Self::Output {
        Watts(self.0 / hours.0)
    }
}

impl From<KilowattHours> for WattHours {
    fn from(kilowatt_hours: KilowattHours) -> Self {
        Self(kilowatt_hours.0 * 1000.0)
    }
}
