use std::ops::Mul;

use fennec_modbus::contrib;

use crate::quantity::{Format, Quantity, energy::MilliwattHours, ratios::BasisPoints};

pub type DecawattHours<V = u32> = Quantity<V, 1, 1, 1, 0>;

impl<V> Format for DecawattHours<V> {
    const SUFFIX: &str = "daWh";
}

impl From<contrib::types::DecawattHours<u16>> for DecawattHours {
    fn from(value: contrib::types::DecawattHours<u16>) -> Self {
        Self(value.0.into())
    }
}

impl From<contrib::types::DecawattHours<u32>> for DecawattHours {
    fn from(value: contrib::types::DecawattHours<u32>) -> Self {
        Self(value.0)
    }
}

impl Mul<BasisPoints> for DecawattHours {
    type Output = MilliwattHours;

    fn mul(self, rhs: BasisPoints) -> Self::Output {
        Quantity(i64::from(self.0) * i64::from(rhs.0))
    }
}
