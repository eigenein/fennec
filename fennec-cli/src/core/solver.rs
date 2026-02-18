use std::time::Instant;

use bon::{Builder, bon};
use chrono::{DateTime, Local, Timelike};
use enumset::EnumSet;

use crate::{
    core::{
        battery,
        energy_level::Quantum,
        solution::Solution,
        solution_space::SolutionSpace,
        step::Step,
        working_mode::{WorkingMode, WorkingModeMap},
    },
    ops::Interval,
    prelude::*,
    quantity::{
        Quantity,
        cost::Cost,
        energy::KilowattHours,
        power::Kilowatts,
        rate::KilowattHourRate,
    },
    statistics::{
        battery::BatteryEfficiency,
        consumption::ConsumptionStatistics,
        flow::{Flow, SystemFlow},
    },
};

#[derive(Builder)]
pub struct Solver<'a> {
    grid_rates: &'a [(Interval, KilowattHourRate)],
    consumption_statistics: &'a ConsumptionStatistics,

    /// Enabled working modes.
    working_modes: EnumSet<WorkingMode>,

    /// Minimum allowed residual energy.
    min_residual_energy: KilowattHours,

    /// Maximum allowed residual energy.
    max_residual_energy: KilowattHours,

    battery_efficiency: BatteryEfficiency,
    purchase_fee: KilowattHourRate,
    degradation_rate: KilowattHourRate,
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
    pub fn solve(self, initial_residual_energy: KilowattHours) -> SolutionSpace {
        let start_instant = Instant::now();

        let max_energy_level = self.quantum.ceil(self.max_residual_energy);
        info!(?self.quantum, ?max_energy_level, n_intervals = self.grid_rates.len(), "optimizing…");

        let minimum_rate =
            self.grid_rates.iter().map(|(_, rate)| *rate).min().unwrap_or(Quantity::ZERO);

        let mut solutions = SolutionSpace::builder()
            .n_intervals(self.grid_rates.len())
            .allowed_energy_levels(
                self.quantum.quantize(self.min_residual_energy)..=max_energy_level,
            )
            .quantum(self.quantum)
            .residual_rate(minimum_rate)
            .initial_residual_energy(initial_residual_energy)
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
                .requested_flows(self.consumption_statistics.on_hour(interval.start.hour()))
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

    pub fn base_loss(&self) -> Cost {
        self.grid_rates
            .iter()
            .copied()
            .map(|(mut interval, grid_rate)| {
                if interval.contains(self.now) {
                    // TODO: de-dup this:
                    interval = interval.with_start(self.now);
                }
                let idle_flow =
                    self.consumption_statistics.on_hour(interval.start.hour())[WorkingMode::Idle];
                self.loss(
                    grid_rate,
                    (idle_flow.grid + idle_flow.battery.reversed()) * interval.len(),
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
        requested_flows: &WorkingModeMap<SystemFlow<Kilowatts>>,
        grid_rate: KilowattHourRate,
        initial_residual_energy: KilowattHours,
        solutions: &SolutionSpace,
    ) -> Option<Solution> {
        let battery = battery::Simulator::builder()
            .residual_energy(initial_residual_energy)
            .min_residual_energy(self.min_residual_energy)
            .max_residual_energy(self.max_residual_energy)
            .efficiency(self.battery_efficiency)
            .build();
        self.working_modes
            .iter()
            .filter_map(|working_mode| {
                let step = self
                    .simulate_step()
                    .interval(interval)
                    .grid_rate(grid_rate)
                    .requested_flow(&requested_flows[working_mode])
                    .initial_residual_energy(initial_residual_energy)
                    .battery(battery)
                    .working_mode(working_mode)
                    .call();
                let next_solution =
                    // Note that the next solution may not exist, hence the question mark:
                    solutions.get(interval_index + 1, step.energy_level_after)?;
                Some(Solution { loss: step.loss + next_solution.loss, step: Some(step) })
            })
            .min_by_key(|solution| solution.loss)
    }

    /// Simulate the battery working in the specified mode given the initial conditions,
    /// and return the loss and new residual energy.
    #[builder]
    fn simulate_step(
        &self,
        mut battery: battery::Simulator,
        interval: Interval,
        requested_flow: &SystemFlow<Kilowatts>,
        grid_rate: KilowattHourRate,
        initial_residual_energy: KilowattHours,
        working_mode: WorkingMode,
    ) -> Step {
        let duration = interval.len();
        let battery_flow = battery.apply(requested_flow.battery, duration);
        let requested_battery = requested_flow.battery * duration;
        let battery_shortage = requested_battery - battery_flow;
        let grid_flow = requested_flow.grid * duration + battery_shortage.reversed();
        Step {
            interval,
            grid_rate,
            working_mode,
            system_flow: SystemFlow { grid: grid_flow, battery: battery_flow },
            residual_energy_after: battery.residual_energy(),
            energy_level_after: self.quantum.quantize(battery.residual_energy()),
            loss: self.loss(grid_rate, grid_flow)
                + (initial_residual_energy - battery.residual_energy()).abs()
                    * self.degradation_rate,
        }
    }

    /// Calculate the grid consumption or production loss.
    fn loss(&self, rate: KilowattHourRate, flow: Flow<KilowattHours>) -> Cost {
        flow.import * rate - flow.export * (rate - self.purchase_fee)
    }
}
