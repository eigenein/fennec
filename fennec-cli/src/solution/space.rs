use std::ops::{Index, IndexMut};

use crate::{
    Schedule,
    energy,
    energy::Flow,
    prelude::*,
    quantity::{energy::EnergyLevel, price::KilowattHourPrice},
    solution::{Backtrack, Optimizer, Solution},
};

pub struct Space {
    /// [Solution space][1] that associates a [`Solution`] with every time interval and [`EnergyLevel`].
    ///
    /// [1]: https://en.wikipedia.org/wiki/Dynamic_programming
    pub solutions: Schedule<Stage>,

    pub optimizer: Optimizer,
}

impl Space {
    /// Re-optimize the solution space at the specified energy level.
    ///
    /// Make sure to the space to the current timestamp.
    pub fn reoptimize_state(
        &mut self,
        initial_energy_level: EnergyLevel,
        energy_profile: &energy::Profile,
    ) {
        self.solutions.get_mut(0)[initial_energy_level] =
            self.optimizer.optimize_state(0, initial_energy_level, energy_profile, &self.solutions);
    }
}

impl Schedule<Stage> {
    pub fn backtrack(&self, initial_energy_level: EnergyLevel) -> Result<Backtrack> {
        let mut energy_level = initial_energy_level;
        let mut metrics = None;

        let schedule = self.try_map(|stage| {
            let solution = stage[energy_level]
                .as_ref()
                .with_context(|| format!("there is no solution at energy level {energy_level}"))?;

            // The first solution carries the cumulative metrics for the entire plan:
            metrics.get_or_insert(solution.metrics);

            energy_level = solution.step.energy_level_after;
            Ok((stage.price, solution.step))
        })?;

        Ok(Backtrack { metrics: metrics.context("the solution space is empty")?, schedule })
    }
}

/// Single stage of the dynamic program: energy price for the time slot
/// and the partial solutions for every energy level.
#[must_use]
pub struct Stage {
    price: Flow<KilowattHourPrice>,

    /// Mapping from [`EnergyLevel`] to a [`Solution`].
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
    pub fn new(price: Flow<KilowattHourPrice>, max_energy_level: EnergyLevel) -> Self {
        Self { price, solutions: vec![None; max_energy_level.0 + 1] }
    }

    pub const fn price(&self) -> Flow<KilowattHourPrice> {
        self.price
    }
}
