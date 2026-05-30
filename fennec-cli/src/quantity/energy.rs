use std::ops::{Div, Mul};

use crate::quantity::{
    power::Watts,
    ratios::{BasisPoints, Percentage},
    time::Hours,
};

quantity!(MilliwattHours, via: i64, suffix: "mWh", precision: 0);
quantity!(WattHours, via: f64, suffix: "Wh", precision: 0);
quantity!(DecawattHours, via: u32, suffix: "daWh", precision: 1);
quantity!(KilowattHours, via: f64, suffix: "kWh", precision: 1);

implement_mul!(Watts, Hours, WattHours);

impl WattHours {
    pub const ONE: Self = Self(1.0);

    /// Project the value into a bucket index.
    ///
    /// TODO: I think I'll eventually use just integer [`WattHours`].
    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    pub const fn index(self, of: Self) -> usize {
        let index = (of.0 / self.0).floor();
        assert!(index >= 0.0);
        index as usize
    }

    /// Un-project the bucket index into the value that represents the middle of the bucket.
    ///
    /// TODO: I think I'll eventually use just integer [`WattHours`].
    #[expect(clippy::cast_precision_loss)]
    pub const fn midpoint(self, index: usize) -> Self {
        Self(self.0 * (index as f64 + 0.5))
    }
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
        MilliwattHours(i64::from(self.0) * i64::from(rhs.0))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_index() {
        assert_eq!(WattHours(1.0).index(WattHours(3.0)), 3);
        assert_eq!(WattHours(1.0).index(WattHours(3.0_f64.next_down())), 2);
    }

    #[test]
    fn energy_midpoint() {
        assert_eq!(WattHours(1.0).midpoint(2), WattHours(2.5));
    }
}
