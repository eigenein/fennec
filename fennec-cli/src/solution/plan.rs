use crate::{
    Schedule,
    energy,
    quantity::price::KilowattHourPrice,
    solution::{Metrics, Step},
};

/// Schedule of working mode decisions along with cumulative metrics.
#[must_use]
pub struct Plan {
    /// Cumulative metrics of the entire plan.
    pub metrics: Metrics,

    pub schedule: Schedule<(energy::Flow<KilowattHourPrice>, Step)>,
}
