use crate::quantity::energy::KilowattHours;

/// Discrete unit of energy used in the solution space of the [`crate::core::solver::Solver`].
#[must_use]
#[derive(Copy, Clone)]
pub struct Quantum(pub KilowattHours);

impl Quantum {
    /// Convert the energy to quantized energy level.
    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    pub fn quantize(self, energy: KilowattHours) -> EnergyLevel {
        debug_assert!(energy >= KilowattHours::ZERO);
        EnergyLevel((energy / self.0).round() as usize)
    }
}

/// Discrete energy level expressed in units of quanta.
#[must_use]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct EnergyLevel(pub usize);

impl EnergyLevel {
    /// Convert the quantized energy level back to conventional energy.
    #[expect(clippy::cast_precision_loss)]
    pub fn dequantize(self, quantizer: Quantum) -> KilowattHours {
        quantizer.0 * (self.0 as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantity::Quantity;

    #[test]
    fn quantize_ok() {
        assert_eq!(Quantum(Quantity(0.084)).quantize(Quantity(0.844)), EnergyLevel(10));
    }

    #[test]
    fn dequantize_ok() {
        assert_eq!(EnergyLevel(10).dequantize(Quantum(Quantity(0.1))), Quantity(1.0));
    }
}
