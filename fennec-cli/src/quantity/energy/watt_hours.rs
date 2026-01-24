use std::fmt::{Debug, Formatter};

use crate::quantity::energy::KilowattHours;

/// Quantized energy for the solver's dynamic programming state space.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct WattHours(pub u32);

impl WattHours {
    pub const ZERO: Self = Self(0);
}

impl Debug for WattHours {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.0}Wh", self.0)
    }
}

impl From<usize> for WattHours {
    fn from(watt_hours: usize) -> Self {
        Self(u32::try_from(watt_hours).expect("watt-hours should fit into `u32`"))
    }
}

impl From<KilowattHours> for WattHours {
    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    fn from(energy: KilowattHours) -> Self {
        Self((energy.0 * 1000.0).max(0.0) as u32)
    }
}

impl From<WattHours> for KilowattHours {
    fn from(watt_hours: WattHours) -> Self {
        Self::from(f64::from(watt_hours.0) / 1000.0)
    }
}

impl From<WattHours> for usize {
    fn from(watt_hours: WattHours) -> Self {
        Self::try_from(watt_hours.0).expect("watt-hours should fit into `usize`")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantity::Quantity;

    #[test]
    fn test_from_positive_kilowatt_hours() {
        assert_eq!(WattHours::from(Quantity::from(1.0)), WattHours(1000));
    }

    #[test]
    fn test_from_negative_kilowatt_hours() {
        assert_eq!(WattHours::from(Quantity::from(-1.0)), WattHours(0));
    }

    #[test]
    fn test_from_watt_hours() {
        assert_eq!(KilowattHours::from(WattHours(1000)), Quantity::from(1.0));
    }
}
