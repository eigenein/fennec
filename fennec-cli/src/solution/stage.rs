use std::ops::{Index, IndexMut};

use crate::{
    energy,
    quantity::{energy::EnergyLevel, price::KilowattHourPrice},
    solution::Solution,
};

/// Single stage of the dynamic program: energy price for the time slot
/// and the partial solutions for every energy level.
#[must_use]
pub struct Stage {
    price: energy::Flow<KilowattHourPrice>,

    /// Mapping from [`EnergyLevel`] to an optional [`Solution`].
    solutions: Vec<Option<Solution>>,
}

impl Index<EnergyLevel> for Stage {
    type Output = Option<Solution>;

    /// Get a reference to the solution at the specified energy level.
    fn index(&self, energy_level: EnergyLevel) -> &Self::Output {
        &self.solutions[energy_level.0]
    }
}

impl IndexMut<EnergyLevel> for Stage {
    /// Get a mutable reference to the solution at the specified energy level.
    fn index_mut(&mut self, energy_level: EnergyLevel) -> &mut Self::Output {
        &mut self.solutions[energy_level.0]
    }
}

impl Stage {
    pub fn new(price: energy::Flow<KilowattHourPrice>, max_energy_level: EnergyLevel) -> Self {
        Self { price, solutions: vec![None; max_energy_level.0 + 1] }
    }

    pub const fn price(&self) -> energy::Flow<KilowattHourPrice> {
        self.price
    }
}
