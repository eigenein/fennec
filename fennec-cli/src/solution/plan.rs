use crate::{
    Schedule,
    energy,
    prelude::*,
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

impl Plan {
    pub fn trace_summary(&self) {
        info!(
            grid_loss = ?self.metrics.losses.grid,
            battery.loss = ?self.metrics.losses.battery,
            battery.charge = ?self.metrics.internal_battery_flow.import,
            battery.discharge = ?self.metrics.internal_battery_flow.export,
            "plan summary",
        );
    }
}
