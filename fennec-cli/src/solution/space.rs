use std::{
    ops::{Index, IndexMut},
    time::Instant,
};

use crate::{
    Schedule,
    api::mini_qube,
    cli::BatteryConfigurationArgs,
    energy,
    energy::Flow,
    prelude::*,
    quantity::{
        Quantity,
        energy::{EnergyLevel, WattHours},
        price::KilowattHourPrice,
    },
    solution::{Metrics, Solution, Step, optimizer::StateOptimizer},
};

pub struct Space<'a> {
    /// [Solution space][1] that associates a [`Solution`] with every time interval and [`EnergyLevel`].
    ///
    /// [1]: https://en.wikipedia.org/wiki/Dynamic_programming
    pub solutions: Schedule<Stage>,

    battery_configuration: &'a BatteryConfigurationArgs,
}

impl<'a> Space<'a> {
    /// Find the optimal battery schedule.
    ///
    /// Works backwards from future to present, computing the minimum cost at each
    /// `(timestamp, residual_energy)` state. Cost is money lost or gained to grid import or export.
    ///
    /// The [DP][1] state space:
    ///
    /// - Time dimension: each hour in the forecast period
    /// - Energy dimension: quantized with the specified step
    ///
    /// For each state, we pick the battery mode that minimizes total cost including future consequences.
    ///
    /// [1]: https://en.wikipedia.org/wiki/Dynamic_programming
    #[instrument(skip_all)]
    pub fn solve(
        energy_prices: &'_ Schedule<Flow<KilowattHourPrice>>,
        energy_profile: &'_ energy::Profile,
        battery_configuration: &'a BatteryConfigurationArgs,
        battery_metrics: &'_ mini_qube::Metrics,
    ) -> Self {
        let start_instant = Instant::now();

        info!(n_intervals = energy_prices.len(), "optimizing…");

        let capacity = battery_metrics.tracked.actual_capacity();
        let capacity_level = EnergyLevel::from(capacity);
        let mut solutions = energy_prices.map(|price| Stage::new(*price, capacity_level));
        let mut n_some: usize = 0;
        let mut n_none: usize = 0;

        // Going backwards:
        for interval_index in (0..solutions.len()).rev() {
            // Calculate partial solutions for the current time interval:
            for energy_level in (0..=capacity_level.0).map(Quantity) {
                let solution =
                    StateOptimizer::new(battery_configuration, battery_metrics, energy_profile)
                        .optimize(interval_index, energy_level, &solutions);
                match solution {
                    Some(_) => n_some += 1,
                    None => n_none += 1,
                }
                solutions.get_mut(interval_index)[energy_level] = solution;
            }
        }

        // TODO: may wanna warn if `n_none` is non-zero.
        info!(elapsed = ?start_instant.elapsed(), n_some, n_none, "optimized");
        Space { solutions, battery_configuration }
    }

    /// Re-optimize the solution space at the specified energy level.
    ///
    /// Make sure to the space to the current timestamp.
    pub fn reoptimize_state(
        &mut self,
        battery_metrics: &mini_qube::Metrics,
        energy_profile: &energy::Profile,
    ) {
        let initial_energy_level =
            WattHours::from(battery_metrics.tracked.residual_energy()).into();
        self.solutions.get_mut(0)[initial_energy_level] =
            StateOptimizer::new(self.battery_configuration, battery_metrics, energy_profile)
                .optimize(0, initial_energy_level, &self.solutions);
    }
}

impl Schedule<Stage> {
    #[expect(clippy::type_complexity)]
    pub fn backtrack(
        &self,
        initial_energy_level: EnergyLevel,
    ) -> Result<(Metrics, Schedule<(Flow<KilowattHourPrice>, Step)>)> {
        let mut energy_level = initial_energy_level;
        let mut metrics = None;

        let steps = self.try_map(|stage| {
            let solution = stage[energy_level]
                .as_ref()
                .with_context(|| format!("there is no solution at energy level {energy_level}"))?;

            // The first solution carries the cumulative metrics for the entire plan:
            metrics.get_or_insert(solution.metrics);

            energy_level = solution.step.energy_level_after;
            Ok((stage.price, solution.step))
        })?;

        let metrics = metrics.context("the solution space is empty")?;
        Ok((metrics, steps))
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
