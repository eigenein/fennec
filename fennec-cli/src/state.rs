use crate::{
    battery,
    quantity::{currency::Mills, power::Watts},
    solution::{Metrics, Step},
};

#[must_use]
pub struct HunterState {
    pub steps: Vec<Step>,
    pub base_loss: Mills,
    pub metrics: Metrics,
    pub average_eps_power: Watts,
}

impl HunterState {
    pub fn profit(&self) -> Mills {
        self.base_loss - self.metrics.losses.total()
    }
}

#[must_use]
pub struct LoggerState {
    pub battery: battery::State,
}
