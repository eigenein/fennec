use musli::{Decode, Encode};

use crate::{
    battery::Efficiency,
    math::smoothing::Exponential,
    quantity::{Zero, power::Watts},
};

/// Battery efficiency estimator.
#[must_use]
#[derive(Encode, Decode)]
pub struct Estimator {
    #[musli(Binary, name = 1)]
    pub charging: Exponential<f64>,

    #[musli(Binary, name = 2)]
    pub discharging: Exponential<f64>,

    #[musli(Binary, name = 3)]
    pub parasitic_load: Exponential<Watts>,
}

impl Default for Estimator {
    fn default() -> Self {
        Self {
            charging: Exponential(0.95),
            discharging: Exponential(0.95),
            parasitic_load: Exponential(Watts::ZERO),
        }
    }
}

impl Estimator {
    pub const fn as_efficiency(&self) -> Efficiency {
        Efficiency {
            charging: self.charging.0,
            discharging: self.discharging.0,
            parasitic_load: self.parasitic_load.0,
        }
    }
}
