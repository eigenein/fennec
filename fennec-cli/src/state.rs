use crate::{
    battery,
    quantity::{currency::Mills, energy::WattHours},
    solution::{Metrics, Step},
};

#[must_use]
pub struct HunterState {
    /// FIXME: this is also present in [`LoggerState`].
    pub actual_capacity: WattHours,

    pub steps: Vec<Step>,
    pub base_loss: Mills,
    pub metrics: Metrics,
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
