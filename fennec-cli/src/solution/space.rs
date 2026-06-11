use std::{
    iter::from_fn,
    ops::{Index, IndexMut},
};

use derive_more::{Deref, DerefMut};

use crate::{
    Schedule,
    energy::Flow,
    prelude::*,
    quantity::{energy::EnergyLevel, price::KilowattHourPrice},
    solution::{Metrics, Solution, Step},
};

/// [Solution space][1] that associates a [`Solution`] with every time interval and [`EnergyLevel`].
///
/// [1]: https://en.wikipedia.org/wiki/Dynamic_programming
#[must_use]
#[derive(Deref, DerefMut)]
pub struct Space(Schedule<Stage>);

impl Space {
    /// TODO: consume `schedule`.
    pub fn new(
        schedule: &Schedule<Flow<KilowattHourPrice>>,
        max_energy_level: EnergyLevel,
    ) -> Self {
        Self(schedule.map(|price| Stage::new(*price, max_energy_level)))
    }

    pub fn backtrack(
        &self,
        initial_energy_level: EnergyLevel,
    ) -> Result<(Metrics, impl Iterator<Item = Step>)> {
        let solution = self.0.get(0).value[initial_energy_level].with_context(|| {
            format!("there is no solution starting at energy level {initial_energy_level}")
        })?;

        // First solution in the chain contains all the cumulative metrics we need:
        let summary = solution.metrics;

        // Unrolling the solution steps:
        let mut next_step = solution.step;
        let mut interval_index = 0;
        let steps = from_fn(move || {
            // Finish when current step is that of the boundary condition:
            let current_step = next_step.take()?;

            // Hop to the next state:
            interval_index += 1;
            if interval_index < self.0.len() {
                // Retrieve the related step if we are not the boundary:
                next_step =
                    // TODO: safety is guaranteed by the algorithm, but can we make it better?
                    self.0.get(interval_index).value[current_step.energy_level_after].unwrap().step;
            }

            // Still yield current step:
            Some(current_step)
        });

        Ok((summary, steps))
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
}
