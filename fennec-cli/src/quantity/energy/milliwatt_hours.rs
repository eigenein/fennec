use crate::quantity::{Format, Quantity};

pub type MilliwattHours<V = i64> = Quantity<V, -3, 1, 1, 0>;

impl<V> Format for MilliwattHours<V> {
    const SUFFIX: &str = "mWh";
}
