use std::fmt::{Display, Formatter};

use crate::quantity::Quantity;

pub type Cost = Quantity<f64, 0, 0, 1>;

impl Display for Cost {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:+.2} €", self.0)
    }
}

impl From<Cost> for opentelemetry::Value {
    fn from(value: Cost) -> Self {
        format!("{:.2}€", value.0).into()
    }
}
