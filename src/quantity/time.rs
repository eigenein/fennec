use crate::quantity::Quantity;

/// FIXME: I should just use `chrono::TimeDelta` instead.
pub type Hours = Quantity<f64, 0, 1, 0>;
