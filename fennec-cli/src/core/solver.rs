use std::time::Instant;

use bon::{Builder, bon};
use chrono::{DateTime, Local, Timelike};
use enumset::EnumSet;

use crate::{
    api::modbus::BatteryState,
    cli::battery::BatteryPowerLimits,
    core::{
        battery::Battery,
        energy_level::{EnergyLevel, Quantum},
        interval::Interval,
        solution::Solution,
        solution_space::SolutionSpace,
        step::Step,
        working_mode::WorkingMode,
    },
    prelude::*,
    quantity::{cost::Cost, energy::KilowattHours, power::Kilowatts, rate::KilowattHourRate},
    statistics::{battery::BatteryEfficiency, consumption::ConsumptionStatistics},
};

#[derive(Builder)]
pub struct Solver<'a> {
    grid_rates: &'a [(Interval, KilowattHourRate)],
    consumption_statistics: &'a ConsumptionStatistics,
    working_modes: EnumSet<WorkingMode>,
    battery_state: BatteryState, // TODO: swap for `BatterySettings` and actual capacity.
    battery_power_limits: BatteryPowerLimits,
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
    pub fn solve(self) -> SolutionSpace {
        let start_instant = Instant::now();

        // TODO: this could be a part of the builder:
        let max_energy =
            self.battery_state.energy.residual().max(self.battery_state.max_residual_energy());
        let max_energy_level = self.quantum.ceil(max_energy);
        info!(?max_energy, ?max_energy_level, n_intervals = self.grid_rates.len(), "optimizing…");

        // TODO: could be a part of the builder:
        let mut solutions = SolutionSpace::new(self.grid_rates.len(), max_energy_level);

        // Going backwards:
        for (interval_index, (mut interval, grid_rate)) in
            self.grid_rates.iter().copied().enumerate().rev()
        {
            if interval.contains(self.now) {
                // The interval has already started, trim the start time:
                interval = interval.with_start(self.now);
            }

            // Average stand-by power at this hour of a day:
            let stand_by_power = self.consumption_statistics.on_hour(interval.start.hour());

            // Calculate partial solutions for the current hour:
            for energy_level in 0..=max_energy_level.0 {
                let energy_level = EnergyLevel(energy_level);
                let initial_residual_energy = energy_level.dequantize(self.quantum);
                *solutions.get_mut(interval_index, energy_level) = self
                    .optimise_step()
                    .interval_index(interval_index)
                    .interval(interval)
                    .stand_by_power(stand_by_power)
                    .grid_rate(grid_rate)
                    .initial_residual_energy(initial_residual_energy)
                    .solutions(&solutions)
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
            .map(|(interval, grid_rate)| {
                let stand_by_power = self.consumption_statistics.on_hour(interval.start.hour());
                self.loss(grid_rate, stand_by_power * interval.duration())
            })
            .sum()
    }

    /// # Returns
    ///
    /// - [`Some`] [`PartialSolution`], if a solution exists.
    /// - [`None`], if there is no solution.
    #[builder]
    fn optimise_step(
        &self,
        interval_index: usize,
        interval: Interval,
        stand_by_power: Kilowatts,
        grid_rate: KilowattHourRate,
        initial_residual_energy: KilowattHours,
        solutions: &SolutionSpace,
    ) -> Option<Solution> {
        let battery = Battery::builder()
            .residual_energy(initial_residual_energy)
            .min_residual_energy(self.battery_state.min_residual_energy())
            .max_residual_energy(self.battery_state.max_residual_energy())
            .efficiency(self.battery_efficiency)
            .build();
        self.working_modes
            .iter()
            .filter_map(|working_mode| {
                let step = self
                    .simulate_step()
                    .interval(interval)
                    .grid_rate(grid_rate)
                    .stand_by_power(stand_by_power)
                    .initial_residual_energy(initial_residual_energy)
                    .battery(battery)
                    .working_mode(working_mode)
                    .call();
                if step.residual_energy_after >= self.battery_state.min_residual_energy() {
                    let next_solution =
                        solutions.get(interval_index + 1, step.energy_level_after)?;
                    Some(Solution {
                        net_loss: step.loss + next_solution.net_loss,
                        charge: step.charge() + next_solution.charge,
                        discharge: step.discharge() + next_solution.discharge,
                        step: Some(step),
                    })
                } else {
                    // Do not allow dropping below the minimally allowed state-of-charge:
                    None
                }
            })
            .min_by_key(|partial_solution| partial_solution.net_loss)
    }

    /// Simulate the battery working in the specified mode given the initial conditions,
    /// and return the loss and new residual energy.
    #[builder]
    fn simulate_step(
        &self,
        mut battery: Battery,
        interval: Interval,
        stand_by_power: Kilowatts,
        grid_rate: KilowattHourRate,
        initial_residual_energy: KilowattHours,
        working_mode: WorkingMode,
    ) -> Step {
        let duration = interval.duration();

        // Requested external power flow to (positive) or from (negative) the battery:
        let battery_external_power = match working_mode {
            WorkingMode::Idle => Kilowatts::ZERO,
            WorkingMode::Backup => (-stand_by_power).max(Kilowatts::ZERO),
            WorkingMode::Charge => self.battery_power_limits.charging_power,
            WorkingMode::Discharge => -self.battery_power_limits.discharging_power,
            WorkingMode::Balance => (-stand_by_power).clamp(
                -self.battery_power_limits.discharging_power,
                self.battery_power_limits.charging_power,
            ),
        };

        // Apply the load to the battery:
        let battery_active_time = battery.apply_load(battery_external_power, duration);

        // Total household energy balance:
        let grid_consumption =
            battery_external_power * battery_active_time + stand_by_power * duration;

        Step {
            interval,
            grid_rate,
            stand_by_power,
            working_mode,
            residual_energy_before: initial_residual_energy,
            residual_energy_after: battery.residual_energy(),
            energy_level_after: self.quantum.quantize(battery.residual_energy()),
            grid_consumption,
            loss: self.loss(grid_rate, grid_consumption)
                + (initial_residual_energy - battery.residual_energy()).abs()
                    * self.degradation_rate,
        }
    }

    /// Calculate the grid consumption or production loss.
    fn loss(&self, grid_rate: KilowattHourRate, consumption: KilowattHours) -> Cost {
        if consumption >= KilowattHours::ZERO {
            consumption * grid_rate
        } else {
            // We sell excess energy cheaper:
            consumption * (grid_rate - self.purchase_fee)
        }
    }
}
