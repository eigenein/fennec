use anyhow::Context;

use crate::{
    Schedule,
    prelude::*,
    quantity::energy::WattHours,
    solution::{Plan, stage::Stage},
};

/// [Solution space][1] that associates a [`super::Solution`] with every time interval and energy level.
///
/// [1]: https://en.wikipedia.org/wiki/Dynamic_programming
pub type Space = Schedule<Stage>;

impl Space {
    /// Recover schedule of working mode decisions starting with the specified residual energy.
    pub fn backtrack(&self, initial_residual_energy: WattHours<usize>) -> Result<Plan> {
        let mut residual_energy = initial_residual_energy;
        let mut metrics = None;

        let schedule = self.try_map(|stage| {
            let solution = stage[residual_energy].as_ref().with_context(|| {
                format!("there is no solution at energy level {residual_energy}")
            })?;

            // The first solution carries the cumulative metrics for the entire plan:
            metrics.get_or_insert(solution.metrics);

            residual_energy = solution.step.residual_energy_after;
            Ok((stage.price(), solution.step))
        })?;

        Ok(Plan { metrics: metrics.context("the solution space is empty")?, schedule })
    }
}
