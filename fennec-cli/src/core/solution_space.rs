use crate::{
    core::{energy_level::EnergyLevel, solution::Solution},
    quantity::energy::KilowattHours,
};

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
    ///
    /// Beyond the last time slot, it will always return [`None`].
    pub fn get(&self, interval_index: usize, energy_level: EnergyLevel) -> Option<&Solution> {
        if interval_index < self.n_intervals {
            self.flat_matrix[self.flat_index(interval_index, energy_level)].as_ref()
        } else {
            None
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
            "index out of bounds: accessed beyond last time slot",
        );
        let flat_index = self.flat_index(interval_index, energy_level);
        &mut self.flat_matrix[flat_index]
    }

    pub fn iter_energy_levels(&self) -> impl Iterator<Item = EnergyLevel> {
        (0..=self.max_energy_level.0).map(EnergyLevel)
    }

    /// Convert the indices into the respective index in the flattened array.
    #[must_use]
    fn flat_index(&self, interval_index: usize, energy_level: EnergyLevel) -> usize {
        debug_assert!(interval_index <= self.n_intervals);
        debug_assert!(energy_level <= self.max_energy_level);
        interval_index * (self.max_energy_level.0 + 1) + energy_level.0
    }
}
