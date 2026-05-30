use crate::quantity::{Format, Quantity};

pub type WattHours<V = f64> = Quantity<V, 0, 1, 1, 0>;

impl Format for WattHours {
    const SUFFIX: &str = "Wh";
}
