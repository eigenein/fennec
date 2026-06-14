use chrono::{DateTime, Local};

use crate::{
    quantity::energy::EnergyLevel,
    solution::{Solution, Solver, Space},
};

pub struct Solved {
    pub space: Space,
    pub solver: Solver,
}

impl Solved {
    /// Re-optimize the state according to the current time and energy level.
    ///
    /// This strips the solution space off the past intervals and re-optimizes starting
    /// with the currently running interval.
    pub fn reoptimize(
        &mut self,
        now: DateTime<Local>,
        initial_energy_level: EnergyLevel,
    ) -> Option<Solution> {
        self.space.pop_before(now);
        if self.space.len() == 0 {
            // TODO: we may need enum since empty space is not the same as missing solution.
            return None;
        }
        self.solver.optimize_state(now, 0, initial_energy_level, &self.space)
    }
}
