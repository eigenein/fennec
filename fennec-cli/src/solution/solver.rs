use std::time::Instant;

use bon::{Builder, bon};
use chrono::{DateTime, Local};
use enumset::EnumSet;

use crate::{
    battery,
    battery::WorkingMode,
    cli::battery::BatteryPowerLimits,
    energy,
    ops::Interval,
    prelude::*,
    quantity::{
        Midpoint,
        Quantum,
        currency::Mills,
        energy::WattHours,
        power::Watts,
        price::KilowattHourPrice,
    },
    solution::{Losses, Metrics, Solution, Space, Step},
};

#[derive(Builder)]
pub struct Solver<'a> {
    energy_prices: &'a [(Interval, KilowattHourPrice)],
    balance_profile: &'a energy::BalanceProfile,

    /// Enabled working modes.
    working_modes: EnumSet<WorkingMode>,

    /// Minimum allowed residual energy.
    min_residual_energy: WattHours,

    /// Maximum allowed residual energy.
    max_residual_energy: WattHours,

    battery_degradation_cost: KilowattHourPrice,
    battery_power_limits: BatteryPowerLimits,
    battery_efficiency: battery::Efficiency,
    purchase_fee: KilowattHourPrice,
    now: DateTime<Local>,
    quantum: WattHours,
}

#[bon]
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

        let min_energy_level = self
            .quantum
            .index(self.min_residual_energy)
            .expect("minimum energy level must be quantizable");
        let max_energy_level = self
            .quantum
            .index(self.max_residual_energy)
            .expect("maximum residual energy must be quantizable");
        info!(?self.quantum, min_energy_level, max_energy_level, n_intervals = self.energy_prices.len(), "optimizing…");

        let mut solutions = Space::builder()
            .n_intervals(self.energy_prices.len())
            .allowed_energy_levels(min_energy_level..=max_energy_level)
            .build();

        // Going backwards:
        for (interval_index, (mut interval, energy_price)) in
            self.energy_prices.iter().copied().enumerate().rev()
        {
            if interval.contains(self.now) {
                // The interval has already started, trim the start time:
                interval = interval.with_start(self.now);
            }

            let optimize_step = self
                .optimize_step()
                .interval_index(interval_index)
                .interval(interval)
                .average_balance(self.balance_profile.on(interval.start.time()))
                .energy_price(energy_price);

            // Calculate partial solutions for the current hour:
            // FIXME: when `interval_index == 0`, we don't need to solve all energy levels.
            for energy_level in 0..=max_energy_level {
                *solutions.get_mut(interval_index, energy_level) = optimize_step
                    .clone()
                    .solutions(&solutions)
                    .initial_residual_energy(self.quantum.midpoint(energy_level))
                    .call();
            }
        }

        info!(elapsed = ?start_instant.elapsed(), "optimized");
        solutions
    }

    pub fn base_loss(&self) -> Mills {
        self.energy_prices
            .iter()
            .copied()
            .map(|(mut interval, energy_price)| {
                if interval.contains(self.now) {
                    // TODO: de-dup this:
                    interval = interval.with_start(self.now);
                }
                let flow = self.balance_profile.on(interval.start.time());
                self.grid_loss(
                    energy_price,
                    (flow.grid + flow.battery.reversed()) * interval.hours(),
                )
            })
            .sum()
    }

    /// # Returns
    ///
    /// - [`Some`] [`PartialSolution`], if a solution exists.
    /// - [`None`], if there is no solution.
    #[builder(derive(Clone))]
    fn optimize_step(
        &self,
        interval_index: usize,
        interval: Interval,
        average_balance: energy::Balance<Watts>,
        energy_price: KilowattHourPrice,
        initial_residual_energy: WattHours,
        solutions: &Space,
    ) -> Option<Solution> {
        let battery = battery::Simulator {
            residual_energy: initial_residual_energy,
            min_residual_energy: self.min_residual_energy,
            max_residual_energy: self.max_residual_energy,
            efficiency: self.battery_efficiency,
        };
        self.working_modes
            .iter()
            .filter_map(|working_mode| {
                let step = self
                    .simulate_step()
                    .interval(interval)
                    .energy_price(energy_price)
                    .average_balance(average_balance)
                    .battery(battery)
                    .working_mode(working_mode)
                    .call();
                let next_solution =
                    // Note that the next solution may not exist, hence the question mark:
                    solutions.get(interval_index + 1, step.energy_level_after)?;
                Some(Solution { metrics: step.metrics + next_solution.metrics, step: Some(step) })
            })
            .min()
    }

    /// Simulate the battery working in the specified mode given the initial conditions,
    /// and return the loss and new residual energy.
    #[builder]
    fn simulate_step(
        &self,
        mut battery: battery::Simulator,
        interval: Interval,
        average_balance: energy::Balance<Watts>,
        energy_price: KilowattHourPrice,
        working_mode: WorkingMode,
    ) -> Step {
        // Remember that the average flow represents theoretical possibility,
        // actual flow depends on the working mode:
        let balance_request =
            average_balance.with_working_mode(working_mode, self.battery_power_limits);
        let hours = interval.hours();
        let battery_flows = battery.apply(balance_request.battery, hours);
        let requested_battery = balance_request.battery * hours;
        let battery_shortage = requested_battery - battery_flows.external;
        let grid_flow = balance_request.grid * hours + battery_shortage.reversed();
        Step {
            interval,
            energy_price,
            working_mode,
            energy_balance: energy::Balance {
                grid: grid_flow.normalized(), // Normalize rare tiny negative values.
                battery: battery_flows.external,
            },
            residual_energy_after: battery.residual_energy,
            energy_level_after: self.quantum.index(battery.residual_energy).unwrap(),
            metrics: Metrics {
                internal_battery_flow: battery_flows.internal,
                losses: Losses {
                    grid: self.grid_loss(energy_price, grid_flow),
                    battery: (battery_flows.internal.import + battery_flows.internal.export)
                        * self.battery_degradation_cost,
                },
            },
        }
    }

    /// Calculate the grid consumption or production loss.
    fn grid_loss(&self, energy_price: KilowattHourPrice, flow: energy::Flow<WattHours>) -> Mills {
        flow.import * energy_price - flow.export * (energy_price - self.purchase_fee)
    }
}
