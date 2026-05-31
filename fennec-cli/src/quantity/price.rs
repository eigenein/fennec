use crate::quantity::{Format, Quantity};

/// TODO: I'm unsure whether `M` should be `3` or `-3`.
pub type KilowattHourPrice<V = f64> = Quantity<V, 3, -1, -1, 1>;

impl Format for KilowattHourPrice {
    const SUFFIX: &str = "¤/kWh";
    const PRECISION: usize = 3;
}
