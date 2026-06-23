use crate::{
    Schedule,
    energy::Flow,
    quantity::price::KilowattHourPrice,
    solution::{Metrics, Step},
};

#[must_use]
pub struct Backtrack {
    /// Solution metrics.
    pub metrics: Metrics,

    /// Schedule of steps taken by the solution.
    pub steps: Schedule<(Flow<KilowattHourPrice>, Step)>,
}
