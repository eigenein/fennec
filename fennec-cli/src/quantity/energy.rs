mod decawatt_hours;
mod milliwatt_hours;

use std::ops::{Div, Mul};

pub use self::{decawatt_hours::DecawattHours, milliwatt_hours::MilliwattHours};
use crate::quantity::{
    Quantity,
    power::Watts,
    ratios::{BasisPoints, Percentage},
    time::Hours,
};

quantity!(WattHours, via: f64, suffix: "Wh", precision: 0);
quantity!(KilowattHours, via: f64, suffix: "kWh", precision: 1);

implement_mul!(Watts, Hours, WattHours);

impl WattHours {
    pub const ONE: Self = Self(1.0);
}

impl From<usize> for WattHours {
    fn from(value: usize) -> Self {
        #[expect(clippy::cast_precision_loss)]
        Self(value as f64)
    }
}

impl From<WattHours> for usize {
    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    fn from(value: WattHours) -> Self {
        value.0 as Self
    }
}

impl From<fennec_modbus::contrib::DecawattHours<u16>> for DecawattHours {
    fn from(value: fennec_modbus::contrib::DecawattHours<u16>) -> Self {
        Self(value.0.into())
    }
}

impl From<fennec_modbus::contrib::DecawattHours<u32>> for DecawattHours {
    fn from(value: fennec_modbus::contrib::DecawattHours<u32>) -> Self {
        Self(value.0)
    }
}

impl Mul<BasisPoints> for DecawattHours {
    type Output = MilliwattHours;

    fn mul(self, rhs: BasisPoints) -> Self::Output {
        Quantity(i64::from(self.0) * i64::from(rhs.0))
    }
}

impl From<DecawattHours> for WattHours {
    fn from(value: DecawattHours) -> Self {
        Self(f64::from(value.0) * 10.0)
    }
}

impl From<MilliwattHours> for WattHours {
    #[expect(clippy::cast_precision_loss)]
    fn from(value: MilliwattHours) -> Self {
        Self((value.0 as f64) * 0.001)
    }
}

impl From<WattHours> for KilowattHours {
    fn from(value: WattHours) -> Self {
        Self(value.0 * 0.001)
    }
}

impl Mul<Percentage> for WattHours {
    type Output = Self;

    fn mul(self, percentage: Percentage) -> Self::Output {
        self * percentage.to_ratio()
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

impl From<DecawattHours> for KilowattHours {
    fn from(decawatt_hours: DecawattHours) -> Self {
        Self(f64::from(decawatt_hours.0) * 0.01)
    }
}
