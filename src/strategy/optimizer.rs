use std::sync::Mutex;

use bon::Builder;
use indicatif::ParallelProgressIterator;
use rayon::prelude::*;

use super::{Forecast, Plan, Solution, Step, WorkingMode, WorkingModeSchedule};
use crate::{
    cache::Cache,
    cli::{BatteryArgs, ConsumptionArgs},
    prelude::*,
    units::{Cost, Hours, KilowattHourRate, KilowattHours, Kilowatts, SurfaceArea},
};

#[derive(Builder)]
pub struct Optimizer<'a> {
    forecast: &'a [Forecast],
    pv_surface_area: SurfaceArea,
    residual_energy: KilowattHours,
    capacity: KilowattHours,
    battery: &'a BatteryArgs,
    consumption: &'a ConsumptionArgs,
    n_steps: usize,
    cache: &'a mut Cache,
}

impl Optimizer<'_> {
    #[instrument(
        name = "Optimising…",
        fields(residual_energy = %self.residual_energy, n_steps = self.n_steps),
        skip_all,
    )]
    pub fn run(self) -> Solution {
        let best_plan: Mutex<(WorkingModeSchedule, Plan)> = {
            let initial_schedule = self.cache.working_mode_schedule;
            Mutex::new((initial_schedule, self.simulate(&initial_schedule)))
        };

        (0..self.n_steps).into_par_iter().progress().for_each(|_| {
            let mut schedule = { best_plan.lock().unwrap().0 };
            schedule.mutate();

            let tested_plan = self.simulate(&schedule);

            let mut best_plan = best_plan.lock().unwrap();
            if tested_plan.net_loss < best_plan.1.net_loss {
                *best_plan = (schedule, tested_plan);
            }
        });

        let (schedule, plan) = best_plan.into_inner().unwrap();
        self.cache.working_mode_schedule = schedule;
        Solution { plan }
    }

    fn simulate(&self, schedule: &WorkingModeSchedule) -> Plan {
        let min_residual_energy = self.capacity * f64::from(self.battery.min_soc_percent) / 100.0;

        let mut current_residual_energy = self.residual_energy;
        let mut steps = Vec::with_capacity(24);

        let mut net_loss = Cost::ZERO;
        let mut net_loss_without_battery = Cost::ZERO;

        for (forecast, working_mode) in self.forecast.iter().copied().zip(schedule.into_iter()) {
            // Apply self-discharge:
            if working_mode == WorkingMode::Retaining {
                current_residual_energy = current_residual_energy * self.battery.retention;
            }

            let initial_residual_energy = current_residual_energy;

            // Positive is excess, negative is deficit:
            let production_power =
                forecast.solar_power_density * self.pv_surface_area - self.consumption.stand_by;

            // Power flow to the battery (negative is directed from the battery):
            let battery_power = match working_mode {
                WorkingMode::Retaining => Kilowatts::ZERO,
                WorkingMode::Charging => self.battery.charging_power,
                WorkingMode::Discharging => -self.battery.discharging_power,
                WorkingMode::Balancing => production_power
                    .clamp(-self.battery.discharging_power, self.battery.charging_power),
            };

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

            let loss = self.loss(forecast.grid_rate, total_consumption);
            net_loss += loss;
            net_loss_without_battery += self.loss(forecast.grid_rate, -production_without_battery);

            steps.push(Step {
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
