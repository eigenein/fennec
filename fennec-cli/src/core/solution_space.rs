use crate::{
    core::{
        energy_level::{EnergyLevel, Quantum},
        solution::Solution,
    },
    quantity::energy::KilowattHours,
};

pub struct SolutionSpace {
    quantum: Quantum,

    /// Energy dimension size.
    max_energy_level: EnergyLevel,

    /// Time dimension size.
    n_intervals: usize,

    /// Flattened 2D array of solutions to speed up the lookups.
    flat_matrix: Vec<Option<Solution>>,
}

impl SolutionSpace {
    pub fn new(quantum: Quantum, n_intervals: usize, max_energy: KilowattHours) -> Self {
        let max_energy_level = quantum.quantize(max_energy);
        let flat_matrix = vec![None; n_intervals * (max_energy_level.0 + 1)];
        Self { quantum, max_energy_level, n_intervals, flat_matrix }
    }

    /// Get the solution at the given time slot index and energy.
    ///
    /// - Energy above the maximum allowed energy will be coerced to the maximum level.
    /// - Beyond the last time slot, it will always return [`None`].
    pub fn get(&self, interval_index: usize, energy: KilowattHours) -> Option<&Solution> {
        if interval_index < self.n_intervals {
            self.flat_matrix[self.flat_index(interval_index, energy)].as_ref()
        } else {
            None
        }
    }

    /// Get the mutable solution at the given time slot index and energy.
    ///
    /// Energy above the maximum allowed energy will be coerced to the maximum level.
    pub fn get_mut(
        &mut self,
        interval_index: usize,
        energy: KilowattHours,
    ) -> &mut Option<Solution> {
        debug_assert!(
            interval_index < self.n_intervals,
            "index out of bounds: accessed beyond last time slot",
        );
        let flat_index = self.flat_index(interval_index, energy);
        &mut self.flat_matrix[flat_index]
    }

    fn flat_index(&self, interval_index: usize, energy: KilowattHours) -> usize {
        let energy_level = self.quantum.quantize(energy).min(self.max_energy_level);
        let flat_index = interval_index * (self.max_energy_level.0 + 1) + energy_level.0;
        debug_assert!(flat_index < self.flat_matrix.len());
        flat_index
    }
}
