mod losses;
mod metrics;

use std::cmp::Ordering;

pub use self::{losses::Losses, metrics::Metrics};
use crate::{core::step::Step, quantity::Zero};

#[must_use]
pub struct Solution {
    pub metrics: Metrics,

    /// First step associated with this solution.
    ///
    /// Boundary solutions have [`None`] here.
    pub step: Option<Step>,
}

impl Solution {
    /// Empty solution that is returned for the time interval beyond the forecast horizon.
    pub const BOUNDARY: Self = Self { metrics: Metrics::ZERO, step: None };
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
