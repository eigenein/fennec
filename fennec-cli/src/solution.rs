mod losses;
mod metrics;
mod optimizer;
mod plan;
mod space;
mod stage;
mod step;

use std::cmp::Ordering;

pub use self::{
    losses::Losses,
    metrics::Metrics,
    optimizer::Optimizer,
    plan::Plan,
    space::Space,
    stage::Stage,
    step::Step,
};

/// Solution for a particular energy level at a particular [`Stage`].
#[must_use]
#[derive(Clone)]
pub struct Solution {
    /// Cumulative metrics of the solution starting with the current stage till the end of plan.
    pub metrics: Metrics,

    /// First step associated with this solution.
    pub step: Step,
}

impl Solution {
    /// Compare this solution total loss to the other solution total loss.
    fn compare_loss_to(&self, other: &Self) -> Ordering {
        self.metrics
            .losses
            .total()
            .partial_cmp(&other.metrics.losses.total())
            .unwrap_or(Ordering::Equal)
    }
}
