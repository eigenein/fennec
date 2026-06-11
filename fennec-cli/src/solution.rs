mod losses;
mod metrics;
mod solver;
mod space;
mod step;

use std::cmp::Ordering;

pub use self::{losses::Losses, metrics::Metrics, solver::Solver, space::Space, step::Step};

#[must_use]
#[derive(Copy, Clone)]
pub struct Solution {
    /// Cumulative metrics across all stages of the solution.
    pub metrics: Metrics,

    /// First step associated with this solution.
    ///
    /// Boundary solutions have [`None`] here.
    pub step: Step,
}

impl Eq for Solution {}

impl PartialEq<Self> for Solution {
    fn eq(&self, other: &Self) -> bool {
        self.metrics.losses.total() == other.metrics.losses.total()
    }
}

impl PartialOrd<Self> for Solution {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Solution {
    fn cmp(&self, other: &Self) -> Ordering {
        self.metrics.losses.total().partial_cmp(&other.metrics.losses.total()).unwrap()
    }
}
