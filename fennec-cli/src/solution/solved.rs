use crate::{
    quantity::energy::EnergyLevel,
    solution::{Solver, Space},
};

pub struct Solved {
    pub space: Space,
    pub solver: Solver,
}

impl Solved {
    /// Re-optimize the solution space at the specified energy level.
    ///
    /// Make sure to the space to the current timestamp.
    pub fn reoptimize_state(&mut self, initial_energy_level: EnergyLevel) {
        self.space.get_mut(0)[initial_energy_level] =
            self.solver.optimize_state(0, initial_energy_level, &self.space);
    }
}
