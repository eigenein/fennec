use std::{cmp::Ordering, iter::from_fn};

use bon::bon;
use itertools::Itertools;

use crate::{
    core::{
        energy_level::EnergyLevel,
        solution::{Metrics, Solution},
        step::Step,
    },
    ops::RangeInclusive,
    prelude::*,
};

#[must_use]
pub struct SolutionSpace {
    allowed_energy_levels: RangeInclusive<EnergyLevel>,

    /// Number of time intervals.
    n_intervals: usize,

    /// Flattened 2D array of solutions to speed up the lookups.
    ///
    /// Here, [`None`] means there is no solution in the given state.
    flat_matrix: Vec<Option<Solution>>,
}

#[bon]
impl SolutionSpace {
    #[builder]
    pub fn new(
        n_intervals: usize,
        #[builder(into)] allowed_energy_levels: RangeInclusive<EnergyLevel>,
    ) -> Self {
        let flat_matrix =
            (0..(n_intervals * (allowed_energy_levels.max.0 + 1))).map(|_| None).collect_vec();
        Self { allowed_energy_levels, n_intervals, flat_matrix }
    }
}

impl SolutionSpace {
    /// Get the solution at the given time slot index and energy.
    ///
    /// This method respects allowed energy levels.
    #[must_use]
    pub fn get(&self, interval_index: usize, energy_level: EnergyLevel) -> Option<&Solution> {
        match interval_index.cmp(&self.n_intervals) {
            Ordering::Less => {
                if self.allowed_energy_levels.contains(energy_level) {
                    self.flat_matrix[self.flat_index(interval_index, energy_level)].as_ref()
                } else {
                    None
                }
            }
            Ordering::Equal => {
                if self.allowed_energy_levels.contains(energy_level) {
                    Some(&Solution::BOUNDARY)
                } else {
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
    /// This method allows accessing any partial solution, regardless of minimally allowed energy levels.
    ///
    /// Panics on energy levels higher than upper bound.
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

    pub fn backtrack(mut self, initial_energy_level: EnergyLevel) -> Result<(Metrics, Vec<Step>)> {
        let solution = self.get_mut(0, initial_energy_level).take().with_context(|| {
            format!("there is no solution starting at energy level {initial_energy_level:?}")
        })?;

        // First solution in the chain contains all the cumulative metrics we need:
        let summary = solution.metrics;

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

        Ok((summary, steps.collect()))
    }

    /// Convert the indices into the respective index in the flattened array.
    ///
    /// Panics on energy levels higher than upper bound.
    #[must_use]
    fn flat_index(&self, interval_index: usize, energy_level: EnergyLevel) -> usize {
        assert!(energy_level <= self.allowed_energy_levels.max);
        interval_index * (self.allowed_energy_levels.max.0 + 1) + energy_level.0
    }
}
