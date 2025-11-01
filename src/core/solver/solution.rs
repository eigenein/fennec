use std::ops::Range;

use chrono::{DateTime, Local};

use crate::{
    core::{series::Series, solver::step::Step},
    quantity::cost::Cost,
};

pub struct Solution {
    pub net_loss: Cost,

    pub net_loss_without_battery: Cost,

    /// The simulated working plan.
    pub steps: Series<Range<DateTime<Local>>, Step>,
}

impl Solution {
    pub fn profit(&self) -> Cost {
        // We expect that with the battery we lose less… 😅
        self.net_loss_without_battery - self.net_loss
    }
}
