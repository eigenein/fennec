use std::cmp::Ordering;

use crate::core::{energy_level::EnergyLevel, solution::Solution};

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
        let flat_matrix = vec![None; n_intervals * (max_energy_level.0 + 1)];
        Self { max_energy_level, n_intervals, flat_matrix }
    }

    /// Get the solution at the given time slot index and energy.
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
    pub fn get_mut(
        &mut self,
        interval_index: usize,
        energy_level: EnergyLevel,
    ) -> &mut Option<Solution> {
        debug_assert!(
            interval_index < self.n_intervals,
            "interval index is out of bounds ({interval_index})",
        );
        let flat_index = self.flat_index(interval_index, energy_level);
        &mut self.flat_matrix[flat_index]
    }

    /// TODO: let's see if I could make it return an iterator later.
    pub fn backtrack(mut self, initial_energy_level: EnergyLevel) -> Option<Vec<Solution>> {
        let mut energy_level = initial_energy_level;
        (0..self.n_intervals)
            .map(|interval_index| {
                let flat_index = self.flat_index(interval_index, energy_level);
                let solution = self.flat_matrix[flat_index].take()?;
                energy_level = solution.step.as_ref()?.energy_level_after;
                Some(solution)
            })
            .collect()
    }

    /// Convert the indices into the respective index in the flattened array.
    #[must_use]
    fn flat_index(&self, interval_index: usize, energy_level: EnergyLevel) -> usize {
        debug_assert!(energy_level <= self.max_energy_level);
        interval_index * (self.max_energy_level.0 + 1) + energy_level.0
    }
}
