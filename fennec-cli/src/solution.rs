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
use crate::quantity::{Zero, currency::Mills};

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
    pub fn total_loss(&self) -> Mills {
        self.metrics.losses.total()
    }

    /// Compare this solution total loss to the other solution total loss.
    fn compare_loss_to(&self, other: &Self) -> Ordering {
        let difference = self.metrics.losses.total() - other.metrics.losses.total();
        if difference.abs() >= Mills::ONE {
            difference.partial_cmp(&Mills::ZERO).unwrap_or(Ordering::Equal)
        } else {
            // Within noise floor – compare actions and prefer lower-action mode:
            self.step.working_mode.cmp(&other.step.working_mode)
        }
    }
}
