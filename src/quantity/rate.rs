use std::fmt::{Display, Formatter};

use crate::quantity::Quantity;

/// Euro per kilowatt-hour.
pub type KilowattHourRate = Quantity<f64, 1, 1, -1>;

impl Display for KilowattHourRate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} €/kWh", self.0)
    }
}

impl From<KilowattHourRate> for opentelemetry::Value {
    fn from(value: KilowattHourRate) -> Self {
        format!("{:.2}€/kWh", value.0).into()
    }
}
