use crate::{
    Schedule,
    energy,
    prelude::*,
    quantity::{energy::DecawattHours, price::KilowattHourPrice},
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
    /// /// Log the plan's headline metrics at `info` level.
    pub fn trace_summary(&self, battery_design_capacity: DecawattHours) {
        let n_cycles = self.metrics.internal_battery_flow.total_throughput()
            / battery_design_capacity.rescale()
            / 2.0;
        info!(
            grid_loss = ?self.metrics.losses.grid,
            battery.loss = ?self.metrics.losses.battery,
            battery.charge = ?self.metrics.internal_battery_flow.import,
            battery.discharge = ?self.metrics.internal_battery_flow.export,
            n_cycles,
            "plan summary",
        );
    }
}
