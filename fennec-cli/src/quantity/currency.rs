use std::ops::Div;

use crate::quantity::{Format, Quantity, price::MillsPerHour, time::Hours};

/// [Mill][1], one-thousandth of the base unit.
///
/// [1]: https://en.wikipedia.org/wiki/Mill_(currency)
pub type Mills<V = f64> = Quantity<V, -3, 0, 0, 1>;

impl<V> Format for Mills<V> {
    const SUFFIX: &str = "₥";
}

impl Mills<f64> {
    /// One cent.
    pub const TEN: Self = Self(10.0);
}

impl Div<Hours> for Mills {
    type Output = MillsPerHour;

    fn div(self, rhs: Hours) -> Self::Output {
        Quantity(self.0 / rhs.0)
    }
}
