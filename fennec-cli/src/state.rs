use crate::{
    quantity::{currency::Mills, energy::WattHours},
    solution::{Metrics, Step},
};

#[must_use]
pub struct SolverState {
    pub actual_capacity: WattHours,
    pub steps: Vec<Step>,
    pub base_loss: Mills,
    pub metrics: Metrics,
}

impl SolverState {
    pub fn profit(&self) -> Mills {
        self.base_loss - self.metrics.losses.total()
    }
}
