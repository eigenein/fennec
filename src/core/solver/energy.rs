use crate::units::energy::KilowattHours;

/// Quantized energy units for the solver's dynamic programming state space.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct DecawattHours(pub u16);

impl From<KilowattHours> for DecawattHours {
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    fn from(energy: KilowattHours) -> Self {
        Self((energy.0 * 100.0).max(0.0) as u16)
    }
}

impl From<DecawattHours> for KilowattHours {
    fn from(energy: DecawattHours) -> Self {
        Self::from(f64::from(energy.0) / 100.0)
    }
}

impl From<DecawattHours> for usize {
    fn from(energy: DecawattHours) -> Self {
        Self::from(energy.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::quantity::Quantity;

    #[test]
    fn test_from_positive_kilowatt_hours() {
        assert_eq!(DecawattHours::from(Quantity(1.0)), DecawattHours(100));
    }

    #[test]
    fn test_from_negative_kilowatt_hours() {
        assert_eq!(DecawattHours::from(Quantity(-1.0)), DecawattHours(0));
    }

    #[test]
    fn test_from_decawatt_hours() {
        assert_eq!(KilowattHours::from(DecawattHours(100)), Quantity(1.0));
    }
}
