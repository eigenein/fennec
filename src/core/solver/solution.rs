use std::rc::Rc;

use crate::{core::solver::PartialSolution, quantity::cost::Cost};

pub struct Solution {
    pub initial_partial_solution: Rc<PartialSolution>,

    /// TODO: move this out of [`Solution`].
    pub net_loss_without_battery: Cost,
}

impl Solution {
    pub fn profit(&self) -> Cost {
        // We expect that with the battery we lose less… 😅
        self.net_loss_without_battery - self.initial_partial_solution.net_loss
    }
}
