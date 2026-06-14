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
    pub fn reoptimize_state(
        &self,
        now: DateTime<Local>,
        initial_energy_level: EnergyLevel,
    ) -> Option<Solution> {
        // FIXME: `now` may be already past the 0th interval.
        self.solver.optimize_state(now, 0, initial_energy_level, &self.space)
    }
}
