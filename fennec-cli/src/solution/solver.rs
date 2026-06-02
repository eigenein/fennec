use std::{range::RangeInclusive, time::Instant};

use bon::Builder;
use chrono::{DateTime, Local};
use enumset::EnumSet;

use crate::{
    Schedule,
    battery,
    battery::WorkingMode,
    energy,
    prelude::*,
    quantity::{Quantity, energy::WattHours, power::Watts, price::KilowattHourPrice},
    solution::{Losses, Metrics, Solution, Space, Step},
};

#[derive(Builder)]
pub struct Solver<'a> {
    energy_prices: &'a Schedule<energy::Flow<KilowattHourPrice>>,
    energy_profile: &'a energy::Profile,
    battery_efficiency: energy::Flow<f64>,
    battery_capacity: WattHours,

    /// Enabled working modes.
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

impl Solver<'_> {
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
    pub fn solve(self) -> Space {
        let start_instant = Instant::now();

        info!(
            allowed_energy_levels =
                ?self.allowed_energy_levels.start..=self.allowed_energy_levels.last,
            n_intervals = self.energy_prices.len(),
            "optimizing…"
        );

        let mut solutions = Space::new(self.energy_prices, self.allowed_energy_levels.last);
        let mut n_some: usize = 0;

        // Going backwards:
        for interval_index in (0..self.energy_prices.len()).rev() {
            // Calculate partial solutions for the current time interval:
            for energy_level in 0..=self.allowed_energy_levels.last.0 {
                let energy_level = Quantity(energy_level);
                *solutions.get_mut(interval_index, energy_level) = self
                    .optimize_step(interval_index, energy_level, &solutions)
                    .inspect(|_| n_some += 1);
            }
        }

        info!(elapsed = ?start_instant.elapsed(), n_some, "optimized");
        solutions
    }

    /// # Returns
    ///
    /// - [`Some`] [`PartialSolution`], if a solution exists.
    /// - [`None`], if there is no solution.
    fn optimize_step(
        &self,
        interval_index: usize,
        initial_residual_energy: impl Into<WattHours>,
        solutions: &Space,
    ) -> Option<Solution> {
        let battery_simulator = battery::Simulator {
            residual_energy: initial_residual_energy.into(),
            capacity: self.battery_capacity,
            efficiency: self.battery_efficiency,
        };
        self.working_modes
            .iter()
            .filter_map(|working_mode| {
                let step = self.simulate_step(battery_simulator, interval_index, working_mode);
                if (step.energy_level_after < self.allowed_energy_levels.start)
                    && !working_mode.is_charging()
                {
                    // Under the minimum allowed energy level disallow anything but charging:
                    return None;
                }
                if (step.energy_level_after > self.allowed_energy_levels.last)
                    && !working_mode.is_discharging()
                {
                    // Above the maximum allowed energy level disallow anything but discharging:
                    // FIXME: buuuut… `step.energy_level_after` may still be out of the space – this was always incorrect.
                    return None;
                }
                // Note that the next solution may not exist, hence the question mark:
                let next_solution = solutions.get(interval_index + 1, step.energy_level_after)?;
                Some(Solution { metrics: step.metrics + next_solution.metrics, step: Some(step) })
            })
            .min()
    }

    /// Simulate the battery working in the specified mode given the initial conditions,
    /// and return the loss and new residual energy.
    fn simulate_step(
        &self,
        mut battery: battery::Simulator,
        interval_index: usize,
        working_mode: WorkingMode,
    ) -> Step {
        let (interval, energy_price) = self.energy_prices.get_unchecked(interval_index);

        let average_balance = self.energy_profile.mean_balance_over(interval);

        // Remember that the average flow represents theoretical possibility,
        // actual flow depends on the working mode:
        let balance_request =
            average_balance.with_working_mode(working_mode, self.max_battery_flow);

        let duration = interval.clamp_start(self.now).duration();
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
            residual_energy_after: battery.residual_energy,
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
