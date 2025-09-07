use std::ops::Mul;

use chrono::TimeDelta;

use crate::units::energy::KilowattHours;

#[derive(
    Copy,
    Clone,
    derive_more::Display,
    derive_more::From,
    derive_more::FromStr,
    derive_more::Neg,
    derive_more::Sub,
    derive_more::Add,
)]
pub struct Kilowatts(pub f64);

impl Kilowatts {
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn into_watts_u32(self) -> u32 {
        (self.0 * 1000.0).round() as u32
    }

    pub fn min(self, rhs: Self) -> Self {
        Self(self.0.min(rhs.0))
    }

    pub fn max(self, rhs: Self) -> Self {
        Self(self.0.max(rhs.0))
    }
}

impl Mul<f64> for Kilowatts {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Mul<TimeDelta> for Kilowatts {
    type Output = KilowattHours;

    fn mul(self, rhs: TimeDelta) -> Self::Output {
        KilowattHours(rhs.as_seconds_f64() / 3600.0 * self.0)
    }
}

pub struct KilowattsPerMeterSquared(pub f64);
