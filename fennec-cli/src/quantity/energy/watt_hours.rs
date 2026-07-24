use std::ops::{Div, Mul};

use crate::quantity::{
    Format,
    Quantity,
    Zero,
    energy::MilliwattHours,
    power::Watts,
    ratios::Percentage,
    time::Hours,
};

pub type WattHours<V = f64> = Quantity<V, 0, 1, 1, 0>;

impl<V> Format for WattHours<V> {
    const SUFFIX: &str = "Wh";
}

/// TODO: generic implementation for any [`Quantity`]:
impl From<WattHours<usize>> for WattHours<f64> {
    fn from(energy_level: WattHours<usize>) -> Self {
        #[expect(clippy::cast_precision_loss)]
        Self(energy_level.0 as f64)
    }
}

/// TODO: generic implementation for any [`Quantity`]:
impl From<WattHours<f64>> for WattHours<usize> {
    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    fn from(value: WattHours) -> Self {
        assert!(value >= WattHours::ZERO);
        // Truncating since rounding could potentially violate the capacity:
        Self(value.0 as usize)
    }
}

impl From<MilliwattHours> for WattHours {
    #[expect(clippy::cast_precision_loss)]
    fn from(value: MilliwattHours) -> Self {
        Self((value.0 as f64) * 0.001)
    }
}

impl Mul<Percentage> for WattHours {
    type Output = Self;

    fn mul(self, percentage: Percentage) -> Self::Output {
        self * percentage.to_ratio()
    }
}

impl Div<Hours> for WattHours {
    type Output = Watts;

    fn div(self, hours: Hours) -> Self::Output {
        Quantity(self.0 / hours.0)
    }
}
