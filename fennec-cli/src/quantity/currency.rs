use crate::quantity::{Format, Quantity, energy::WattHours, price::KilowattHourPrice};

/// [Mill][1], one-thousandth of the base unit.
///
/// [1]: https://en.wikipedia.org/wiki/Mill_(currency)
pub type Mills<V = f64> = Quantity<V, -3, 0, 0, 1>;

impl Format for Mills {
    const SUFFIX: &str = "₥";
}

implement_mul!(KilowattHourPrice, WattHours, Mills);

impl Mills {
    /// One cent.
    pub const TEN: Self = Self(10.0);
}
