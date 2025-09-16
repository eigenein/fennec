use std::sync::Mutex;

use bon::Builder;
use indicatif::ParallelProgressIterator;
use rayon::prelude::*;

use super::{Metrics, Point, Solution, Step, WorkingMode};
use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    prelude::*,
    units::{Cost, Hours, KilowattHourRate, KilowattHours, Kilowatts, Quantity, SurfaceArea},
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
    pub fn run(self, initial_schedule: Vec<Point<WorkingMode>>) -> Solution {
        let best_solution: Mutex<(Vec<Point<WorkingMode>>, Solution)> = {
            let initial_solution = self.simulate(&initial_schedule);
            Mutex::new((initial_schedule, initial_solution))
        };

        (0..self.n_steps).into_par_iter().progress().for_each(|_| {
            let mut schedule = { best_solution.lock().unwrap().0.clone() };
            Self::mutate(&mut schedule);

            let solution = self.simulate(&schedule);

            let mut best_solution = best_solution.lock().unwrap();
            if solution.net_loss < best_solution.1.net_loss {
                *best_solution = (schedule, solution);
            }
        });

        best_solution.into_inner().unwrap().1
    }

    fn mutate(schedule: &mut [Point<WorkingMode>]) {
        const MODES: [WorkingMode; 4] = [
            WorkingMode::Idle,
            WorkingMode::Balancing,
            WorkingMode::Charging,
            WorkingMode::Discharging,
        ];
        for _ in 0..2 {
            schedule[fastrand::usize(0..schedule.len())].value = fastrand::choice(MODES).unwrap();
        }
    }

    fn simulate(&self, schedule: &[Point<WorkingMode>]) -> Solution {
        let min_residual_energy = self.capacity * f64::from(self.battery.min_soc_percent) / 100.0;

        let mut current_residual_energy = self.residual_energy;
        let mut steps = Vec::with_capacity(self.metrics.len());

        let mut net_loss = Cost::ZERO;
        let mut net_loss_without_battery = Cost::ZERO;

        for (metrics, working_mode) in self.metrics.iter().zip(schedule) {
            assert_eq!(metrics.time, working_mode.time);
            let initial_residual_energy = current_residual_energy;

            // For missing weather forecast, assume none solar power:
            let solar_production =
                metrics.value.solar_power_density.unwrap_or(Quantity::ZERO) * self.pv_surface_area;
            // Positive is excess, negative is deficit:
            let power_balance = solar_production - self.consumption.stand_by;

            // Power flow to the battery (negative is directed from the battery):
            let battery_external_power = match working_mode.value {
                WorkingMode::Idle => Kilowatts::ZERO,
                WorkingMode::Charging => self.battery.charging_power,
                WorkingMode::Discharging => -self.battery.discharging_power,
                WorkingMode::Balancing => power_balance
                    .clamp(-self.battery.discharging_power, self.battery.charging_power),
            };

            // Power flow inside the battery corrected by the round-trip efficiency:
            let battery_external_consumption = if battery_external_power
                > self.battery.self_discharge
            {
                // While charging, the residual energy grows slower:
                let internal_power = battery_external_power * self.battery.efficiency;
                current_residual_energy = (current_residual_energy + internal_power * Hours::ONE)
                    .min(self.capacity.max(initial_residual_energy));
                let time_charging =
                    (current_residual_energy - initial_residual_energy) / internal_power;
                assert!(time_charging >= Hours::ZERO);
                battery_external_power * time_charging
            } else if battery_external_power < -self.battery.self_discharge {
                // While discharging, the residual energy is spent faster:
                let internal_power = battery_external_power / self.battery.efficiency;
                // Remember that the power here is negative:
                current_residual_energy = (current_residual_energy + internal_power * Hours::ONE)
                    .max(min_residual_energy.min(initial_residual_energy));
                let time_discharging =
                    (current_residual_energy - initial_residual_energy) / internal_power;
                assert!(time_discharging >= Hours::ZERO);
                battery_external_power * time_discharging
            } else {
                // Idle and self-discharging:
                current_residual_energy = (current_residual_energy
                    - self.battery.self_discharge * Hours::ONE)
                    .max(KilowattHours::ZERO);
                KilowattHours::ZERO
            };

            // Finally, total household energy balance:
            let production_without_battery = power_balance * Hours::ONE;
            let grid_consumption = battery_external_consumption - production_without_battery;

            let loss = self.loss(metrics.value.grid_rate, grid_consumption);
            net_loss += loss;
            net_loss_without_battery +=
                self.loss(metrics.value.grid_rate, -production_without_battery);

            steps.push(Point {
                time: metrics.time,
                value: Step {
                    working_mode: working_mode.value,
                    residual_energy_before: initial_residual_energy,
                    residual_energy_after: current_residual_energy,
                    grid_consumption,
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
