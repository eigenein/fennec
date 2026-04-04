use crate::{
    battery,
    energy,
    quantity::currency::Mills,
    solution::{Metrics, Step},
};

#[must_use]
pub struct HunterState {
    pub steps: Vec<Step>,
    pub base_loss: Mills,
    pub metrics: Metrics,
    pub energy_profile: energy::Profile,
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
