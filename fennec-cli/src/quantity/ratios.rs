use std::ops::Mul;

use crate::{
    prelude::*,
    quantity::{Format, Quantity},
};

pub type Percentage<V = u8> = Quantity<V, 2, 0, 0, 0>;

impl<V> Format for Percentage<V> {
    const SUFFIX: &str = "%";
}

pub type BasisPoints<V = u16> = Quantity<V, 3, 0, 0, 0>;

impl<V> Format for BasisPoints<V> {
    const SUFFIX: &str = "‱";
}

impl From<Percentage> for fennec_modbus::contrib::Percentage<u8> {
    fn from(percentage: Percentage) -> Self {
        Self(percentage.0)
    }
}

impl From<Percentage> for fennec_modbus::contrib::Percentage<u16> {
    fn from(percentage: Percentage) -> Self {
        Self(percentage.0.into())
    }
}

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
        Quantity(u16::from(self.0) * u16::from(rhs.0))
    }
}
