use anyhow::Context;

use crate::{
    Schedule,
    prelude::*,
    quantity::energy::EnergyLevel,
    solution::{Optimizer, Plan, stage::Stage},
};

/// [Solution space][1] that associates a [`super::Solution`] with every time interval and [`EnergyLevel`].
///
/// [1]: https://en.wikipedia.org/wiki/Dynamic_programming
pub type Space = Schedule<Stage>;

impl Space {
    /// Recover schedule of working mode decisions starting with the specified [`EnergyLevel`].
    pub fn backtrack(&self, initial_energy_level: EnergyLevel) -> Result<Plan> {
        let mut energy_level = initial_energy_level;
        let mut metrics = None;

        let schedule = self.try_map(|stage| {
            let solution = stage[energy_level]
                .as_ref()
                .with_context(|| format!("there is no solution at energy level {energy_level}"))?;

            // The first solution carries the cumulative metrics for the entire plan:
            metrics.get_or_insert(solution.metrics);

            energy_level = solution.step.energy_level_after;
            Ok((stage.price(), solution.step))
        })?;

        Ok(Plan { metrics: metrics.context("the solution space is empty")?, schedule })
    }

    /// Re-optimize the solution space at the specified energy level.
    ///
    /// Make sure to advance the schedule to the current timestamp.
    pub fn reoptimize_state(&mut self, optimizer: &Optimizer, initial_energy_level: EnergyLevel) {
        self.get_mut(0)[initial_energy_level] =
            optimizer.optimize_state(0, initial_energy_level, self);
    }
}
