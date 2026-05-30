use musli::{Decode, Encode};

use crate::{
    math::smoothing::Exponential,
    quantity::{Zero, power::Watts},
};

/// Battery efficiency estimator.
#[must_use]
#[derive(Clone, Encode, Decode)]
pub struct Efficiency {
    #[musli(Binary, name = 1)]
    pub charging: Exponential<f64>,

    #[musli(Binary, name = 2)]
    pub discharging: Exponential<f64>,

    #[musli(Binary, name = 3)]
    pub parasitic_load: Exponential<Watts>,
}

impl Default for Efficiency {
    fn default() -> Self {
        Self {
            charging: Exponential(0.95),
            discharging: Exponential(0.95),
            parasitic_load: Exponential(Watts::ZERO),
        }
    }
}

impl Efficiency {
    #[cfg(test)]
    pub const IDEAL: Self = Self {
        charging: Exponential(1.0),
        discharging: Exponential(1.0),
        parasitic_load: Exponential(Watts::ZERO),
    };

    pub const fn round_trip(&self) -> f64 {
        self.charging.0 * self.discharging.0
    }
}
