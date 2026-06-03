use std::{cmp::Ordering, iter::from_fn};

use crate::{
    Schedule,
    prelude::*,
    quantity::energy::EnergyLevel,
    solution::{Metrics, Solution, Step},
};

/// [Dynamic programming][1] solution space.
///
/// [1]: https://en.wikipedia.org/wiki/Dynamic_programming
#[must_use]
pub struct Space(Schedule<Vec<Option<Solution>>>);

impl Space {
    pub fn new<V>(schedule: &Schedule<V>, max_energy_level: EnergyLevel) -> Self {
        Self(schedule.map(|_| vec![None; max_energy_level.0 + 1]))
    }

    /// Get the solution at the given time slot index and energy.
    #[must_use]
    pub fn get(&self, interval_index: usize, energy_level: EnergyLevel) -> Option<&Solution> {
        match interval_index.cmp(&self.0.len()) {
            Ordering::Less => self.0.get(interval_index).1[energy_level.0].as_ref(),
            Ordering::Equal => Some(&Solution::BOUNDARY),
            Ordering::Greater => panic!("interval index is out of bounds ({interval_index})"),
        }
    }

    /// Get the mutable solution at the given time slot index and energy.
    ///
    /// Panics outside the bounds.
    #[must_use]
    pub fn get_mut(
        &mut self,
        interval_index: usize,
        energy_level: EnergyLevel,
    ) -> &mut Option<Solution> {
        &mut self.0.get_mut(interval_index)[energy_level.0]
    }

    pub fn backtrack(
        &self,
        initial_energy_level: EnergyLevel,
    ) -> Result<(Metrics, impl Iterator<Item = Step>)> {
        let solution = self.0.get(0).1[initial_energy_level.0].with_context(|| {
            format!("there is no solution starting at energy level {initial_energy_level}")
        })?;

        // First solution in the chain contains all the cumulative metrics we need:
        let summary = solution.metrics;

        // Unrolling the solution steps:
        let mut next_step = solution.step;
        let mut interval_index = 0;
        let steps = from_fn(move || {
            // Finish when current step is that of the boundary condition:
            let current_step = next_step.take()?;

            // Hop to the next state:
            interval_index += 1;
            if interval_index < self.0.len() {
                // Retrieve the related step if we are not the boundary:
                next_step = self.0.get(interval_index).1[current_step.energy_level_after.0]
                    .expect("next energy level must point to an existing solution")
                    .step;
            }

            // Still yield current step:
            Some(current_step)
        });

        Ok((summary, steps))
    }
}
