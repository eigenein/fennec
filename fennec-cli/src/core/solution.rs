mod summary;

use std::cmp::Ordering;

pub use self::summary::Summary;
use crate::{
    core::step::Step,
    quantity::{Zero, currency::Mills},
};

#[must_use]
pub struct Solution {
    /// Cumulative loss to the grid till the end of the forecast period
    pub grid_loss: Mills,

    /// First step associated with this solution.
    ///
    /// Boundary solutions have [`None`] here.
    pub step: Option<Step>,
}

impl Solution {
    /// Empty solution that is returned for the time interval beyond the forecast horizon.
    pub const BOUNDARY: Self = Self { grid_loss: Mills::ZERO, step: None };
}

impl Eq for Solution {}

impl PartialEq<Self> for Solution {
    fn eq(&self, other: &Self) -> bool {
        self.grid_loss == other.grid_loss
    }
}

impl PartialOrd<Self> for Solution {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Solution {
    fn cmp(&self, other: &Self) -> Ordering {
        self.grid_loss.partial_cmp(&other.grid_loss).unwrap()
    }
}
