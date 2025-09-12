use std::ops::Mul;

use crate::units::{Kilowatts, Quantity, SurfaceArea};

/// [Surface power density][1] measured in **kilowatts per meter squared**.
///
/// [1]: https://en.wikipedia.org/wiki/Surface_power_density
pub type PowerDensity = Quantity<f64, 1, -2, 0, 0>;

impl Mul<SurfaceArea> for PowerDensity {
    type Output = Kilowatts;

    fn mul(self, rhs: SurfaceArea) -> Self::Output {
        Quantity(self.0 * rhs.0)
    }
}
