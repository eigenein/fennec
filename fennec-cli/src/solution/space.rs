use std::{cmp::Ordering, iter::from_fn, ops::RangeInclusive};

use bon::bon;
use grid::Grid;

use crate::{
    battery::WorkingMode,
    prelude::*,
    solution::{Metrics, Solution, Step},
};

#[must_use]
pub struct Space {
    inner: Grid<Option<Solution>>,
    allowed_energy_levels: RangeInclusive<usize>,
}

#[bon]
impl Space {
    #[builder]
    pub fn new(n_intervals: usize, allowed_energy_levels: RangeInclusive<usize>) -> Self {
        Self {
            inner: Grid::new(n_intervals, allowed_energy_levels.end() + 1),
            allowed_energy_levels,
        }
    }
}

impl Space {
    /// Get the solution at the given time slot index and energy.
    #[must_use]
    pub fn get(
        &self,
        interval_index: usize,
        energy_level: usize,
        working_mode: WorkingMode,
    ) -> Option<&Solution> {
        match interval_index.cmp(&self.inner.rows()) {
            Ordering::Less => {
                if (
                    // Normal operation:
                    self.allowed_energy_levels.contains(&energy_level)
                ) || (
                    // From under the allowed energy levels, only allow charging:
                    (energy_level < *self.allowed_energy_levels.start())
                        && working_mode.is_charging()
                ) || (
                    // From above the allowed energy levels, only allow discharging:
                    (energy_level > *self.allowed_energy_levels.end())
                        && working_mode.is_discharging()
                ) {
                    self.inner[(interval_index, energy_level)].as_ref()
                } else {
                    // Invalid energy level.
                    None
                }
            }
            Ordering::Equal => {
                if self.allowed_energy_levels.contains(&energy_level) {
                    Some(&Solution::BOUNDARY)
                } else {
                    // Invalid energy level.
                    None
                }
            }
            Ordering::Greater => {
                panic!("interval index is out of bounds ({interval_index})");
            }
        }
    }

    /// Get the mutable solution at the given time slot index and energy.
    ///
    /// Panics outside the bounds.
    #[must_use]
    pub fn get_mut(&mut self, interval_index: usize, energy_level: usize) -> &mut Option<Solution> {
        &mut self.inner[(interval_index, energy_level)]
    }

    pub fn backtrack(&self, initial_energy_level: usize) -> Result<(Metrics, Vec<Step>)> {
        let solution = self.inner[(0, initial_energy_level)].with_context(|| {
            format!("there is no solution starting at energy level {initial_energy_level}")
        })?;

        // First solution in the chain contains all the cumulative metrics we need:
        let summary = solution.metrics;

        // Unrolling the solution steps:
        let mut next_step = solution.step;
        let mut interval_index = 0;
        let steps = from_fn(|| {
            // Finish when current step is that of the boundary condition:
            let current_step = next_step.take()?;

            // Hop to the next state:
            interval_index += 1;
            if interval_index < self.inner.rows() {
                // Retrieve the related step if we are not the boundary:
                next_step = self.inner[(interval_index, current_step.energy_level_after)]
                    .expect("next energy level must point to an existing solution")
                    .step;
            }

            // Still yield current step:
            Some(current_step)
        });

        Ok((summary, steps.collect()))
    }
}
