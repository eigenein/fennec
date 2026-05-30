use musli::{Decode, Encode};

use crate::{
    math::smoothing::Exponential,
    quantity::{Zero, power::Watts},
};

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
            charging: Exponential::new(0.95),
            discharging: Exponential::new(0.95),
            parasitic_load: Exponential::new(Watts::ZERO),
        }
    }
}
