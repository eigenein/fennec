use std::fmt::{Debug, Display, Formatter};

use crate::quantity::Quantity;

/// Euro per kilowatt-hour.
pub type KilowattHourRate = Quantity<1, 1, -1>;

impl Display for KilowattHourRate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} €/kWh", self.0)
    }
}

impl Debug for KilowattHourRate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}€/kWh", self.0)
    }
}
