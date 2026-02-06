use std::{cmp::Ordering, iter::from_fn};

use itertools::Itertools;

use crate::{
    core::{
        energy_level::EnergyLevel,
        solution::{CumulativeMetrics, Solution},
        step::Step,
    },
    prelude::*,
};

#[must_use]
pub struct SolutionSpace {
    /// Energy dimension size.
    max_energy_level: EnergyLevel,

    /// Time dimension size.
    n_intervals: usize,

    /// Flattened 2D array of solutions to speed up the lookups.
    ///
    /// Here, [`None`] means there is no solution in the given state.
    flat_matrix: Vec<Option<Solution>>,
}

impl SolutionSpace {
    pub fn new(n_intervals: usize, max_energy_level: EnergyLevel) -> Self {
        let flat_matrix = (0..(n_intervals * (max_energy_level.0 + 1))).map(|_| None).collect_vec();
        Self { max_energy_level, n_intervals, flat_matrix }
    }

    /// Get the solution at the given time slot index and energy.
    #[must_use]
    pub fn get(&self, interval_index: usize, energy_level: EnergyLevel) -> Option<&Solution> {
        match interval_index.cmp(&self.n_intervals) {
            Ordering::Less => {
                self.flat_matrix[self.flat_index(interval_index, energy_level)].as_ref()
            }
            Ordering::Equal => Some(&Solution::BOUNDARY),
            Ordering::Greater => {
                panic!("interval index is out of bounds ({interval_index})");
            }
        }
    }

    /// Get the mutable solution at the given time slot index and energy.
    #[must_use]
    pub fn get_mut(
        &mut self,
        interval_index: usize,
        energy_level: EnergyLevel,
    ) -> &mut Option<Solution> {
        match interval_index.cmp(&self.n_intervals) {
            Ordering::Less => {
                let flat_index = self.flat_index(interval_index, energy_level);
                &mut self.flat_matrix[flat_index]
            }
            Ordering::Equal => {
                panic!("boundary solutions are immutable");
            }
            Ordering::Greater => {
                panic!("interval index is out of bounds ({interval_index})");
            }
        }
    }

    pub fn backtrack(
        mut self,
        initial_energy_level: EnergyLevel,
    ) -> Result<(CumulativeMetrics, Vec<Step>)> {
        let solution = self.get_mut(0, initial_energy_level).take().with_context(|| {
            format!("there is no solution starting at energy level {initial_energy_level:?}")
        })?;

        // Cumulative metrics of the first entry is the metrics of the entire chain:
        let metrics = solution.cumulative_metrics;

        // Unrolling the solution steps:
        let mut step = solution.step;
        let mut interval_index = 0;
        let steps = from_fn(|| {
            // Finish when current step is that of the boundary condition:
            let current_step = step.take()?;

            // Hop to the next state:
            let next_energy_level = current_step.energy_level_after;
            interval_index += 1;
            if interval_index < self.n_intervals {
                // Retrieve the related step if we are not the boundary:
                step = self
                    .get_mut(interval_index, next_energy_level)
                    .take()
                    .expect("next energy level must point to an existing solution")
                    .step;
            }

            // Still yield current step:
            Some(current_step)
        });

        Ok((metrics, steps.collect()))
    }

    /// Convert the indices into the respective index in the flattened array.
    #[must_use]
    fn flat_index(&self, interval_index: usize, energy_level: EnergyLevel) -> usize {
        debug_assert!(energy_level <= self.max_energy_level);
        interval_index * (self.max_energy_level.0 + 1) + energy_level.0
    }
}
