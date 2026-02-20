use std::time::Instant;

use bon::{Builder, bon};
use chrono::{DateTime, Local, Timelike};
use enumset::EnumSet;

use crate::{
    cli::battery::BatteryPowerLimits,
    core::{
        battery,
        energy_level::Quantum,
        flow::{EnergyBalance, Flow},
        solution::{Losses, Solution},
        solution_space::SolutionSpace,
        step::Step,
        working_mode::WorkingMode,
    },
    ops::Interval,
    prelude::*,
    quantity::{currency::Mills, energy::WattHours, power::Watts, rate::KilowattHourRate},
    statistics::{FlowStatistics, battery::BatteryEfficiency},
};

#[derive(Builder)]
pub struct Solver<'a> {
    grid_rates: &'a [(Interval, KilowattHourRate)],
    flow_statistics: &'a FlowStatistics,

    /// Enabled working modes.
    working_modes: EnumSet<WorkingMode>,

    /// Minimum allowed residual energy.
    min_residual_energy: WattHours,

    /// Maximum allowed residual energy.
    max_residual_energy: WattHours,

    battery_degradation_rate: KilowattHourRate,
    battery_power_limits: BatteryPowerLimits,
    battery_efficiency: BatteryEfficiency,
    purchase_fee: KilowattHourRate,
    now: DateTime<Local>,
    quantum: Quantum,
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
    /// - Energy dimension: quantized to 10 Wh increments (decawatt-hours)
    ///
    /// For each state, we pick the battery mode that minimizes total cost including future consequences.
    ///
    /// [1]: https://en.wikipedia.org/wiki/Dynamic_programming
    #[instrument(skip_all)]
    pub fn solve(self) -> SolutionSpace {
        let start_instant = Instant::now();

        let max_energy_level = self.quantum.ceil(self.max_residual_energy);
        info!(?self.quantum, ?max_energy_level, n_intervals = self.grid_rates.len(), "optimizing…");

        let mut solutions = SolutionSpace::builder()
            .n_intervals(self.grid_rates.len())
            .allowed_energy_levels(
                self.quantum.quantize(self.min_residual_energy)..=max_energy_level,
            )
            .build();

        // Going backwards:
        for (interval_index, (mut interval, grid_rate)) in
            self.grid_rates.iter().copied().enumerate().rev()
        {
            if interval.contains(self.now) {
                // The interval has already started, trim the start time:
                interval = interval.with_start(self.now);
            }

            let optimize_step = self
                .optimize_step()
                .interval_index(interval_index)
                .interval(interval)
                .average_balance(self.flow_statistics.on_hour(interval.start.hour()))
                .grid_rate(grid_rate);

            // Calculate partial solutions for the current hour:
            // FIXME: when `interval_index == 0`, we don't need to solve all energy levels.
            for energy_level in max_energy_level.iter_from_zero() {
                *solutions.get_mut(interval_index, energy_level) = optimize_step
                    .clone()
                    .solutions(&solutions)
                    .initial_residual_energy(energy_level.dequantize(self.quantum))
                    .call();
            }
        }

        info!(elapsed = ?start_instant.elapsed(), "optimized");
        solutions
    }

    pub fn base_loss(&self) -> Mills {
        self.grid_rates
            .iter()
            .copied()
            .map(|(mut interval, grid_rate)| {
                if interval.contains(self.now) {
                    // TODO: de-dup this:
                    interval = interval.with_start(self.now);
                }
                let flow = self.flow_statistics.on_hour(interval.start.hour());
                self.grid_loss(grid_rate, (flow.grid + flow.battery.reversed()) * interval.hours())
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
        average_balance: EnergyBalance<Watts>,
        grid_rate: KilowattHourRate,
        initial_residual_energy: WattHours,
        solutions: &SolutionSpace,
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
                    .grid_rate(grid_rate)
                    .average_balance(average_balance)
                    .battery(battery)
                    .working_mode(working_mode)
                    .call();
                let next_solution =
                    // Note that the next solution may not exist, hence the question mark:
                    solutions.get(interval_index + 1, step.energy_level_after)?;
                Some(Solution { losses: step.losses + next_solution.losses, step: Some(step) })
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
        average_balance: EnergyBalance<Watts>,
        grid_rate: KilowattHourRate,
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
            grid_rate,
            working_mode,
            energy_balance: EnergyBalance { grid: grid_flow, battery: battery_flows.external },
            residual_energy_after: battery.residual_energy,
            energy_level_after: self.quantum.quantize(battery.residual_energy),
            losses: Losses {
                grid: self.grid_loss(grid_rate, grid_flow),
                battery: (battery_flows.internal.import + battery_flows.internal.export)
                    * self.battery_degradation_rate,
            },
        }
    }

    /// Calculate the grid consumption or production loss.
    fn grid_loss(&self, rate: KilowattHourRate, flow: Flow<WattHours>) -> Mills {
        flow.import * rate - flow.export * (rate - self.purchase_fee)
    }
}
