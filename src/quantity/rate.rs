use std::fmt::{Display, Formatter};

use crate::quantity::Quantity;

/// Euro per kilowatt-hour.
pub type KilowattHourRate = Quantity<f64, 1, 1, -1>;

impl Display for KilowattHourRate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} €/kWh", self.0)
    }
}
