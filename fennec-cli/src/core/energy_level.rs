use derive_more::{From, FromStr};

use crate::quantity::{Zero, energy::WattHours};

/// Discrete unit of energy used in the solution space of the [`crate::core::solver::Solver`].
#[must_use]
#[derive(Copy, Clone, From, FromStr, derive_more::Debug)]
#[debug("{_0:?}")]
pub struct Quantum(pub WattHours);

impl Quantum {
    /// Convert the energy to quantized energy level.
    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    pub fn quantize(self, energy: WattHours) -> EnergyLevel {
        debug_assert!(energy >= WattHours::ZERO);
        EnergyLevel((energy / self.0).round() as usize)
    }

    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    pub fn ceil(self, energy: WattHours) -> EnergyLevel {
        debug_assert!(energy >= WattHours::ZERO);
        EnergyLevel((energy / self.0).ceil() as usize)
    }
}

/// Discrete energy level expressed in units of quanta.
#[must_use]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, derive_more::Debug)]
#[debug("{_0}")]
pub struct EnergyLevel(pub usize);

impl EnergyLevel {
    /// Convert the quantized energy level back to conventional energy.
    #[expect(clippy::cast_precision_loss)]
    pub fn dequantize(self, quantizer: Quantum) -> WattHours {
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

    #[test]
    fn quantize_ok() {
        assert_eq!(Quantum(WattHours(84.0)).quantize(WattHours(844.0)), EnergyLevel(10));
    }

    #[test]
    fn dequantize_ok() {
        assert_eq!(EnergyLevel(10).dequantize(Quantum(WattHours(100.0))), WattHours(1000.0));
    }
}
