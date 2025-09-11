use std::sync::Mutex;

use bon::Builder;
use indicatif::ParallelProgressIterator;
use rayon::prelude::*;

use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    prelude::*,
    strategy::WorkingMode,
    units::{Cost, Hours, KilowattHourRate, KilowattHours, Kilowatts},
};

#[derive(Builder)]
pub struct Optimizer<'a> {
    hourly_rates: &'a [KilowattHourRate],
    solar_power: &'a [Kilowatts],
    residual_energy: KilowattHours,
    capacity: KilowattHours,
    battery: &'a BatteryArgs,
    consumption: &'a ConsumptionArgs,
    n_steps: usize,
}

impl Optimizer<'_> {
    #[instrument(name = "Optimising…", fields(residual_energy = %self.residual_energy), skip_all)]
    pub fn run(self) -> Solution {
        let n_hours = self.hourly_rates.len().min(self.solar_power.len());
        let best_plan: Mutex<(Vec<WorkingMode>, Option<Plan>)> =
            Mutex::new((vec![WorkingMode::Maintaining; n_hours], None));

        (0..self.n_steps).into_par_iter().progress().for_each(|_| {
            let mut working_modes = { best_plan.lock().unwrap().0.clone() };
            for _ in 0..2 {
                let new_mode = fastrand::choice([
                    WorkingMode::Maintaining,
                    WorkingMode::Balancing,
                    WorkingMode::Charging,
                    WorkingMode::Discharging,
                ]);
                working_modes[fastrand::usize(0..n_hours)] = new_mode.unwrap();
            }

            let tested_plan = self.simulate(&working_modes);

            let mut best_plan = best_plan.lock().unwrap();
            if best_plan
                .1
                .as_ref()
                .is_none_or(|best_plan| tested_plan.net_loss < best_plan.net_loss)
            {
                *best_plan = (working_modes, Some(tested_plan));
            }
        });

        Solution { plan: best_plan.into_inner().unwrap().1.unwrap() }
    }

    fn simulate(&self, working_modes: &[WorkingMode]) -> Plan {
        let min_residual_energy = self.capacity * f64::from(self.battery.min_soc_percent) / 100.0;

        let mut current_residual_energy = self.residual_energy;
        let mut steps = Vec::with_capacity(self.hourly_rates.len());

        let mut net_loss = Cost::ZERO;
        let mut net_loss_without_battery = Cost::ZERO;

        for ((grid_rate, solar_power), working_mode) in self
            .hourly_rates
            .iter()
            .copied()
            .zip(self.solar_power.iter().copied())
            .zip(working_modes.iter().copied())
        {
            // Positive is excess, negative is deficit:
            let production_power = solar_power - self.consumption.stand_by;

            // Power flow to the battery (negative is directed from the battery):
            let battery_power = match working_mode {
                WorkingMode::Maintaining => Kilowatts::ZERO,
                WorkingMode::Charging => self.battery.charging_power,
                WorkingMode::Discharging => -self.battery.discharging_power,
                WorkingMode::Balancing => production_power
                    .clamp(-self.battery.discharging_power, self.battery.charging_power),
            };

            let initial_residual_energy = current_residual_energy;
            current_residual_energy = (initial_residual_energy + battery_power * Hours::ONE).clamp(
                min_residual_energy.min(initial_residual_energy),
                self.capacity.max(initial_residual_energy),
            );

            // Positive is consumption while charging, negative is production while discharging:
            let battery_internal_consumption = current_residual_energy - initial_residual_energy;
            let battery_external_consumption = if battery_internal_consumption > KilowattHours::ZERO
            {
                // While charging, we consume more due to inefficiency:
                battery_internal_consumption / self.battery.efficiency
            } else if battery_internal_consumption < KilowattHours::ZERO {
                // While discharging, we produce less:
                battery_internal_consumption * self.battery.efficiency
            } else {
                KilowattHours::ZERO
            };

            // Finally, total household energy balance:
            let production_without_battery = production_power * Hours::ONE;
            let total_consumption = battery_external_consumption - production_without_battery;

            let loss = self.loss(grid_rate, total_consumption);
            net_loss += loss;
            net_loss_without_battery += self.loss(grid_rate, -production_without_battery);

            steps.push(HourStep {
                working_mode,
                residual_energy_before: initial_residual_energy,
                residual_energy_after: current_residual_energy,
                total_consumption,
                loss,
            });
        }

        Plan { net_loss, net_loss_without_battery, steps }
    }

    fn loss(&self, grid_rate: KilowattHourRate, consumption: KilowattHours) -> Cost {
        if consumption >= KilowattHours::ZERO {
            consumption * grid_rate
        } else {
            // We sell excess energy cheaper:
            consumption * (grid_rate - self.consumption.purchase_fees)
        }
    }
}

/// Optimization plan that describes how the battery will work in the upcoming hours.
pub struct Plan {
    pub net_loss: Cost,
    pub net_loss_without_battery: Cost,
    pub steps: Vec<HourStep>,
}

impl Plan {
    pub fn profit(&self) -> Cost {
        // We expect that with the battery we lose less… 😅
        self.net_loss_without_battery - self.net_loss
    }
}

/// Single-hour working plan step.
pub struct HourStep {
    pub working_mode: WorkingMode,
    pub residual_energy_before: KilowattHours,
    pub residual_energy_after: KilowattHours,
    pub total_consumption: KilowattHours,
    pub loss: Cost,
}

pub struct Solution {
    pub plan: Plan,
}
