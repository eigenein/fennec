use crate::quantity::{Format, Quantity};

pub type DecawattHours<V = u32> = Quantity<V, 1, 1, 1, 0>;

impl Format for DecawattHours {
    const SUFFIX: &str = "daWh";
}
