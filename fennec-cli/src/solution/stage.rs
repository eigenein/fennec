use std::ops::{Index, IndexMut};

use crate::{
    energy,
    quantity::{energy::WattHours, price::KilowattHourPrice},
    solution::Solution,
};

/// Single stage of the dynamic program: energy price for the time slot
/// and the partial solutions for every energy level.
#[must_use]
pub struct Stage {
    price: energy::Flow<KilowattHourPrice>,

    /// Mapping from residual energy to an optional [`Solution`].
    solutions: Vec<Option<Solution>>,
}

impl Index<WattHours<usize>> for Stage {
    type Output = Option<Solution>;

    /// Get a reference to the solution at the specified energy level.
    fn index(&self, residual_energy: WattHours<usize>) -> &Self::Output {
        &self.solutions[residual_energy.0]
    }
}

impl IndexMut<WattHours<usize>> for Stage {
    /// Get a mutable reference to the solution at the specified energy level.
    fn index_mut(&mut self, residual_energy: WattHours<usize>) -> &mut Self::Output {
        &mut self.solutions[residual_energy.0]
    }
}

impl Stage {
    pub fn new(price: energy::Flow<KilowattHourPrice>, battery_capacity: WattHours<usize>) -> Self {
        Self { price, solutions: vec![None; battery_capacity.0 + 1] }
    }

    pub const fn price(&self) -> energy::Flow<KilowattHourPrice> {
        self.price
    }
}
