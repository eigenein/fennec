use std::sync::Mutex;

use bon::Builder;
use indicatif::ParallelProgressIterator;
use itertools::Itertools;
use rayon::prelude::*;

use super::{Metrics, Point, Solution, Step, WorkingMode};
use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    prelude::*,
    units::{Cost, Hours, KilowattHourRate, KilowattHours, Kilowatts, SurfaceArea},
};

#[derive(Builder)]
pub struct Optimizer<'a> {
    metrics: &'a [Point<Metrics>],
    pv_surface_area: SurfaceArea,
    residual_energy: KilowattHours,
    capacity: KilowattHours,
    battery: BatteryArgs,
    consumption: ConsumptionArgs,
    n_steps: usize,
}

impl Optimizer<'_> {
    #[instrument(
        name = "Optimising…",
        fields(residual_energy = %self.residual_energy, n_steps = self.n_steps),
        skip_all,
    )]
    pub fn run(self) -> Solution {
        let best_solution: Mutex<(Vec<Point<WorkingMode>>, Solution)> = {
            // TODO: fill in from the cache:
            let initial_schedule = self
                .metrics
                .iter()
                .map(|point| Point { time: point.time, value: WorkingMode::default() })
                .collect_vec();
            let initial_solution = self.simulate(&initial_schedule);
            Mutex::new((initial_schedule, initial_solution))
        };

        (0..self.n_steps).into_par_iter().progress().for_each(|_| {
            let mut schedule = { best_solution.lock().unwrap().0.clone() };
            Self::mutate(&mut schedule);

            let trial = self.simulate(&schedule);

            let mut best_solution = best_solution.lock().unwrap();
            if trial.net_loss < best_solution.1.net_loss {
                *best_solution = (schedule, trial);
            }
        });

        let (_, plan) = best_solution.into_inner().unwrap();
        plan
    }

    fn mutate(schedule: &mut [Point<WorkingMode>]) {
        for point in schedule.iter_mut() {
            if fastrand::u8(0..10) == 0 {
                point.value = fastrand::choice([
                    WorkingMode::Retaining,
                    WorkingMode::Balancing,
                    WorkingMode::Charging,
                    WorkingMode::Discharging,
                ])
                .unwrap();
            }
        }
    }

    fn simulate(&self, schedule: &[Point<WorkingMode>]) -> Solution {
        let min_residual_energy = self.capacity * f64::from(self.battery.min_soc_percent) / 100.0;

        let mut current_residual_energy = self.residual_energy;
        let mut steps = Vec::with_capacity(self.metrics.len());

        let mut net_loss = Cost::ZERO;
        let mut net_loss_without_battery = Cost::ZERO;

        let series =
            self.metrics.iter().zip(schedule).inspect(|(lhs, rhs)| assert_eq!(lhs.time, rhs.time));
        for (metrics, working_mode) in series {
            // Apply self-discharge:
            current_residual_energy = current_residual_energy * self.battery.retention;

            let initial_residual_energy = current_residual_energy;

            // Positive is excess, negative is deficit:
            let production_power = metrics.value.solar_power_density * self.pv_surface_area
                - self.consumption.stand_by;

            // Power flow to the battery (negative is directed from the battery):
            let battery_power = match working_mode.value {
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

            let loss = self.loss(metrics.value.grid_rate, total_consumption);
            net_loss += loss;
            net_loss_without_battery +=
                self.loss(metrics.value.grid_rate, -production_without_battery);

            steps.push(Point {
                time: metrics.time,
                value: Step {
                    working_mode: working_mode.value,
                    residual_energy_before: initial_residual_energy,
                    residual_energy_after: current_residual_energy,
                    total_consumption,
                    loss,
                },
            });
        }

        Solution { net_loss, net_loss_without_battery, steps }
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
