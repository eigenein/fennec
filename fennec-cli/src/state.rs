use crate::{quantity::energy::WattHours, solution::Step};

#[must_use]
pub struct SolverState {
    pub actual_capacity: WattHours,
    pub steps: Vec<Step>,
}
