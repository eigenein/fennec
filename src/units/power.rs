use std::ops::Mul;

use crate::units::{Hours, KilowattHours, Quantity};

pub type Kilowatts = Quantity<f64, 1, 0, 0, 0>;

impl Kilowatts {
    #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn into_watts_u32(self) -> u32 {
        (self.0 * 1000.0).round() as u32
    }

    pub const fn min(self, rhs: Self) -> Self {
        Self(self.0.min(rhs.0))
    }

    pub const fn max(self, rhs: Self) -> Self {
        Self(self.0.max(rhs.0))
    }
}

impl Mul<Hours> for Kilowatts {
    type Output = KilowattHours;

    fn mul(self, rhs: Hours) -> Self::Output {
        Quantity(self.0 * rhs.0)
    }
}
