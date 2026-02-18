use std::fmt::{Debug, Formatter};

use derive_more::{From, FromStr};

use crate::quantity::energy::KilowattHours;

/// Discrete unit of energy used in the solution space of the [`crate::core::solver::Solver`].
#[must_use]
#[derive(Copy, Clone, From, FromStr)]
#[from(KilowattHours)]
pub struct Quantum(pub KilowattHours);

impl Debug for Quantum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Quantum {
    /// Convert the energy to quantized energy level.
    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    pub fn quantize(self, energy: KilowattHours) -> EnergyLevel {
        debug_assert!(energy >= KilowattHours::ZERO);
        EnergyLevel((energy / self.0).round() as usize)
    }

    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    pub fn ceil(self, energy: KilowattHours) -> EnergyLevel {
        debug_assert!(energy >= KilowattHours::ZERO);
        EnergyLevel((energy / self.0).ceil() as usize)
    }
}

/// Discrete energy level expressed in units of quanta.
#[must_use]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct EnergyLevel(pub usize);

impl Debug for EnergyLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl EnergyLevel {
    /// Convert the quantized energy level back to conventional energy.
    #[expect(clippy::cast_precision_loss)]
    pub fn dequantize(self, quantizer: Quantum) -> KilowattHours {
        quantizer.0 * (self.0 as f64)
    }

    /// Iterate through the energy levels starting with zero and ending with the current level.
    pub fn iter_from_zero(self) -> impl Iterator<Item = Self> {
        (0..=self.0).map(Self)
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
