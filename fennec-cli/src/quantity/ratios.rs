use std::ops::Mul;

use crate::prelude::*;

quantity!(Percentage, via: u8, suffix: "%", precision: 1);
quantity!(BasisPoints, via: u16, suffix: "‱", precision: 0);

impl TryFrom<fennec_modbus::contrib::Percentage<u16>> for Percentage {
    type Error = Error;

    fn try_from(value: fennec_modbus::contrib::Percentage<u16>) -> Result<Self> {
        Ok(Self(value.0.try_into()?))
    }
}

impl Percentage {
    /// Convert the percentage into `0.0..=1.0`.
    pub const fn to_ratio(self) -> f64 {
        0.01 * self.0 as f64
    }
}

impl Mul<Self> for Percentage {
    type Output = BasisPoints;

    fn mul(self, rhs: Self) -> Self::Output {
        BasisPoints(u16::from(self.0) * u16::from(rhs.0))
    }
}
