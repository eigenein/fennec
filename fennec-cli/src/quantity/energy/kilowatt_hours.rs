use crate::quantity::{Format, Quantity};

pub type KilowattHours<V = f64> = Quantity<V, 3, 1, 1, 0>;

impl<V> Format for KilowattHours<V> {
    const SUFFIX: &str = "kWh";
    const PRECISION: usize = 1;
}
