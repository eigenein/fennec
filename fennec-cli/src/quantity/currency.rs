use crate::quantity::{energy::WattHours, price::KilowattHourPrice};

quantity!(
    /// [Mill][1], one-thousandth of the base unit.
    ///
    /// [1]: https://en.wikipedia.org/wiki/Mill_(currency)
    Mills, via: f64, suffix: "₥", precision: 0
);

implement_mul!(KilowattHourPrice, WattHours, Mills);

impl Mills {
    /// One cent.
    pub const TEN: Self = Self(10.0);
}
