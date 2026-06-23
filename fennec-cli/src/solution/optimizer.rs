use std::{range::RangeInclusive, time::Instant};

use bon::Builder;

use crate::{
    Schedule,
    battery,
    battery::WorkingMode,
    energy::{Balance, Flow, Profile},
    prelude::*,
    quantity::{
        Quantity,
        energy::{EnergyLevel, WattHours},
        power::Watts,
        price::KilowattHourPrice,
        time::Hours,
    },
    schedule::Slot,
    solution::{Losses, Metrics, Optimized, Solution, Step, optimized::Stage},
};

#[derive(Builder)]
pub struct Optimizer {
    battery_capacity: WattHours,

    /// Enabled working modes.
    working_modes: Vec<WorkingMode>,

    /// Incurred cost of the residual energy change per kilowatt-hour.
    battery_degradation_cost: KilowattHourPrice,

    /// Maximum power flow that the battery supports.
    max_battery_flow: Flow<Watts>,

    /// Allowed energy level range.
    #[builder(into)]
    allowed_energy_levels: RangeInclusive<WattHours<usize>>,
}

impl Optimizer {
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
        self,
        energy_prices: &Schedule<Flow<KilowattHourPrice>>,
        energy_profile: &Profile,
    ) -> Optimized {
        let start_instant = Instant::now();

        info!(?self.allowed_energy_levels, n_intervals = energy_prices.len(), "optimizing…");

        let mut solutions =
            energy_prices.map(|price| Stage::new(*price, self.allowed_energy_levels.last));
        let mut n_some: usize = 0;
        let mut n_none: usize = 0;

        // Going backwards:
        for interval_index in (0..solutions.len()).rev() {
            // Calculate partial solutions for the current time interval:
            // FIXME: calculate up to the capacity:
            for energy_level in (0..=self.allowed_energy_levels.last.0).map(Quantity) {
                let solution =
                    self.optimize_state(interval_index, energy_level, energy_profile, &solutions);
                match solution {
                    Some(_) => n_some += 1,
                    None => n_none += 1,
                }
                solutions.get_mut(interval_index)[energy_level] = solution;
            }
        }

        // TODO: may wanna warn if `n_none` is non-zero.
        info!(elapsed = ?start_instant.elapsed(), n_some, n_none, "optimized");
        Optimized { solutions, optimizer: self }
    }

    /// # Returns
    ///
    /// - [`Some`] [`PartialSolution`], if a solution exists.
    /// - [`None`], if there is no solution.
    pub fn optimize_state(
        &self,
        interval_index: usize,
        initial_energy_level: EnergyLevel,
        energy_profile: &Profile,
        solutions: &Schedule<Stage>,
    ) -> Option<Solution> {
        let Slot { interval, value: stage } = solutions.get(interval_index);
        let duration = interval.duration();
        let average_balance = energy_profile.mean_balance_over(interval);
        let battery_simulator = battery::Simulator {
            residual_energy: initial_energy_level.into(),
            capacity: self.battery_capacity,
            efficiency: energy_profile.battery_efficiency,
        };
        self.working_modes
            .iter()
            .filter_map(|working_mode| {
                let step = self.simulate_step(
                    battery_simulator,
                    duration,
                    average_balance,
                    stage.price(),
                    *working_mode,
                );
                if (step.energy_level_after < initial_energy_level)
                    && (initial_energy_level <= self.allowed_energy_levels.start)
                {
                    // At or under the minimum allowed energy level, forbid going lower:
                    return None;
                }
                if (step.energy_level_after > initial_energy_level)
                    && (initial_energy_level >= self.allowed_energy_levels.last)
                {
                    // At or above the maximum allowed energy level, forbid going higher:
                    return None;
                }

                let mut metrics = step.metrics;
                let next_interval_index = interval_index + 1;

                if next_interval_index < solutions.len() {
                    // For non-boundary solutions, accumulate the target optimization metrics:
                    metrics += solutions.get(next_interval_index).value[step.energy_level_after]
                        .as_ref()?
                        .metrics;
                }

                Some(Solution { metrics, step })
            })
            .min()
    }

    /// Simulate the battery working in the specified mode given the initial conditions.
    fn simulate_step(
        &self,
        mut battery: battery::Simulator,
        duration: Hours,
        average_balance: Balance<Watts>,
        energy_price: Flow<KilowattHourPrice>,
        working_mode: WorkingMode,
    ) -> Step {
        // Remember that the average flow represents theoretical possibility,
        // actual flow depends on the working mode:
        let balance_request =
            average_balance.with_working_mode(working_mode, self.max_battery_flow);

        let battery_flows = battery.apply(balance_request.battery, duration);
        let requested_battery = balance_request.battery * duration;
        let battery_shortage = requested_battery - battery_flows.external;
        let grid_flow = balance_request.grid * duration + battery_shortage.reversed();
        Step {
            working_mode,
            duration,
            energy_balance: Balance {
                grid: grid_flow.normalized(), // Normalize rare tiny negative values.
                battery: battery_flows.external,
            },
            energy_level_after: battery.residual_energy.into(),
            metrics: Metrics {
                internal_battery_flow: battery_flows.internal,
                losses: Losses {
                    grid: energy_price.loss(grid_flow),
                    battery: (battery_flows.internal.import + battery_flows.internal.export)
                        * self.battery_degradation_cost,
                },
            },
        }
    }
}
