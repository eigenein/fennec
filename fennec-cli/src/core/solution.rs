mod losses;
mod summary;

use std::cmp::Ordering;

pub use self::{losses::Losses, summary::Summary};
use crate::{core::step::Step, quantity::Zero};

#[must_use]
pub struct Solution {
    pub losses: Losses,

    /// First step associated with this solution.
    ///
    /// Boundary solutions have [`None`] here.
    pub step: Option<Step>,
}

impl Solution {
    /// Empty solution that is returned for the time interval beyond the forecast horizon.
    pub const BOUNDARY: Self = Self { losses: Losses::ZERO, step: None };
}

impl Eq for Solution {}

impl PartialEq<Self> for Solution {
    fn eq(&self, other: &Self) -> bool {
        self.losses.total() == other.losses.total()
    }
}

impl PartialOrd<Self> for Solution {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Solution {
    fn cmp(&self, other: &Self) -> Ordering {
        self.losses.total().partial_cmp(&other.losses.total()).unwrap()
    }
}
