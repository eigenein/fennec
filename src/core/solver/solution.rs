use crate::{
    core::{series::Series, solver::step::Step},
    quantity::{cost::Cost, time_range::TimeRange},
};

pub struct Solution {
    pub net_loss: Cost,

    /// TODO: move this out of [`Solution`].
    pub net_loss_without_battery: Cost,

    /// The simulated working plan.
    pub steps: Series<TimeRange, Step>,
}

impl Solution {
    pub fn profit(&self) -> Cost {
        // We expect that with the battery we lose less… 😅
        self.net_loss_without_battery - self.net_loss
    }
}
