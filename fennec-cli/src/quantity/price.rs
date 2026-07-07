use crate::quantity::{Format, Quantity};

/// Energy price in "¤/kWh" – which is effectively the same as "₥/Wh".
pub type KilowattHourPrice<V = f64> = Quantity<V, -3, -1, -1, 1>;

impl Format for KilowattHourPrice {
    const SUFFIX: &str = "¤/kWh";
    const PRECISION: usize = 3;
}
