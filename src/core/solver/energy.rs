use crate::quantity::energy::KilowattHours;

/// Quantized energy for the solver's dynamic programming state space.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct WattHours(pub u32);

impl From<KilowattHours> for WattHours {
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    fn from(energy: KilowattHours) -> Self {
        Self((energy.0 * 1000.0).max(0.0) as u32)
    }
}

impl From<WattHours> for KilowattHours {
    fn from(energy: WattHours) -> Self {
        Self::from(f64::from(energy.0) / 1000.0)
    }
}

impl From<WattHours> for usize {
    fn from(energy: WattHours) -> Self {
        Self::try_from(energy.0).expect("the energy level should fit into `usize`")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantity::Quantity;

    #[test]
    fn test_from_positive_kilowatt_hours() {
        assert_eq!(WattHours::from(Quantity(1.0)), WattHours(1000));
    }

    #[test]
    fn test_from_negative_kilowatt_hours() {
        assert_eq!(WattHours::from(Quantity(-1.0)), WattHours(0));
    }

    #[test]
    fn test_from_decawatt_hours() {
        assert_eq!(KilowattHours::from(WattHours(1000)), Quantity(1.0));
    }
}
