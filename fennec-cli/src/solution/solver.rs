use std::{range::RangeInclusive, time::Instant};

use bon::Builder;
use chrono::{DateTime, Local};
use enumset::EnumSet;

use crate::{
    Schedule,
    battery,
    battery::WorkingMode,
    energy,
    energy::Flow,
    ops::chrono::Interval,
    prelude::*,
    quantity::{
        Quantity,
        energy::{EnergyLevel, WattHours},
        power::Watts,
        price::KilowattHourPrice,
    },
    schedule::Slot,
    solution::{Losses, Metrics, Solution, Space, Step},
};

#[derive(Builder)]
pub struct Solver {
    energy_profile: energy::Profile,
    battery_efficiency: energy::Flow<f64>,
    battery_capacity: WattHours,

    /// Enabled working modes.
    ///
    /// TODO: do we need [`EnumSet`]?
    working_modes: EnumSet<WorkingMode>,

    /// Incurred cost of the residual energy change per kilowatt-hour.
    battery_degradation_cost: KilowattHourPrice,

    /// Maximum power flow that the battery supports.
    max_battery_flow: energy::Flow<Watts>,

    /// Allowed energy level range.
    #[builder(into)]
    allowed_energy_levels: RangeInclusive<WattHours<usize>>,

    now: DateTime<Local>,
}

impl Solver {
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
    pub fn solve(self, energy_prices: Schedule<energy::Flow<KilowattHourPrice>>) -> Space {
        let start_instant = Instant::now();

        info!(?self.allowed_energy_levels, n_intervals = energy_prices.len(), "optimizing…");

        let mut solutions = Space::new(energy_prices, self.allowed_energy_levels.last);
        let mut n_some: usize = 0;
        let mut n_none: usize = 0;

        // Going backwards:
        for interval_index in (0..solutions.len()).rev() {
            // Calculate partial solutions for the current time interval:
            // FIXME: calculate up to the capacity:
            for energy_level in (0..=self.allowed_energy_levels.last.0).map(Quantity) {
                let solution = self.optimize_state(interval_index, energy_level, &solutions);
                match solution {
                    Some(_) => n_some += 1,
                    None => n_none += 1,
                }
                solutions.get_mut(interval_index)[energy_level] = solution;
            }
        }

        // TODO: may wanna warn if `n_none` is non-zero.
        info!(elapsed = ?start_instant.elapsed(), n_some, n_none, "optimized");
        solutions
    }

    /// # Returns
    ///
    /// - [`Some`] [`PartialSolution`], if a solution exists.
    /// - [`None`], if there is no solution.
    fn optimize_state(
        &self,
        interval_index: usize,
        initial_energy_level: EnergyLevel,
        solutions: &Space,
    ) -> Option<Solution> {
        let Slot { interval, value: stage } = solutions.get(interval_index);
        let battery_simulator = battery::Simulator {
            residual_energy: initial_energy_level.into(),
            capacity: self.battery_capacity,
            efficiency: self.battery_efficiency,
        };
        self.working_modes
            .iter()
            .filter_map(|working_mode| {
                let step =
                    self.simulate_step(battery_simulator, interval, stage.price(), working_mode);
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

    /// Simulate the battery working in the specified mode given the initial conditions,
    /// and return the loss and new residual energy.
    fn simulate_step(
        &self,
        mut battery: battery::Simulator,
        interval: Interval,
        price: Flow<KilowattHourPrice>,
        working_mode: WorkingMode,
    ) -> Step {
        let interval = interval.clamp_start_to(self.now);

        let average_balance = self.energy_profile.mean_balance_over(interval);

        // Remember that the average flow represents theoretical possibility,
        // actual flow depends on the working mode:
        let balance_request =
            average_balance.with_working_mode(working_mode, self.max_battery_flow);

        let duration = interval.duration();
        let battery_flows = battery.apply(balance_request.battery, duration);
        let requested_battery = balance_request.battery * duration;
        let battery_shortage = requested_battery - battery_flows.external;
        let grid_flow = balance_request.grid * duration + battery_shortage.reversed();
        Step {
            working_mode,
            duration,
            energy_balance: energy::Balance {
                grid: grid_flow.normalized(), // Normalize rare tiny negative values.
                battery: battery_flows.external,
            },
            energy_level_after: battery.residual_energy.into(),
            metrics: Metrics {
                internal_battery_flow: battery_flows.internal,
                losses: Losses {
                    grid: price.loss(grid_flow),
                    battery: (battery_flows.internal.import + battery_flows.internal.export)
                        * self.battery_degradation_cost,
                },
            },
        }
    }
}
