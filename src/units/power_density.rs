use crate::units::Quantity;

/// [Surface power density][1] measured in **kilowatts per meter squared**.
///
/// [1]: https://en.wikipedia.org/wiki/Surface_power_density
pub type PowerDensity = Quantity<f64, 1, -2, 0, 0>;
