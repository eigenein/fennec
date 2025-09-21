use bon::Builder;
use chrono::{DateTime, Local};
use indicatif::ProgressIterator;

use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    core::{
        metrics::Metrics,
        series::Series,
        solution::Solution,
        step::Step,
        summary::Summary,
        working_mode::WorkingMode,
    },
    prelude::*,
    units::{
        currency::Cost,
        energy::KilowattHours,
        power::Kilowatts,
        quantity::Quantity,
        rate::KilowattHourRate,
        surface_area::SquareMetres,
        time::Hours,
    },
};

#[derive(Builder)]
pub struct Optimizer<'a> {
    metrics: &'a Series<Metrics>,
    pv_surface_area: SquareMetres,
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
    pub fn run(self, initial_schedule: Series<WorkingMode>) -> Result<(usize, Solution)> {
        let mut n_mutations_succeeded = 0;
        let mut best_schedule = initial_schedule;
        let mut step_buffer = Vec::with_capacity(self.metrics.len());
        let mut best_solution = Solution {
            summary: self.simulate(&best_schedule, &mut step_buffer)?,
            steps: step_buffer.clone(),
        };
        (0..self.n_steps).progress().try_for_each(|_| {
            self.step(
                &mut best_schedule,
                &mut best_solution,
                &mut step_buffer,
                &mut n_mutations_succeeded,
            )
        })?;
        Ok((n_mutations_succeeded, best_solution))
    }

    fn step(
        &self,
        schedule: &mut Series<WorkingMode>,
        best_solution: &mut Solution,
        step_buffer: &mut Vec<(DateTime<Local>, Step)>,
        n_mutations_succeeded: &mut usize,
    ) -> Result {
        let (mutation_1, mutation_2) = schedule.mutate();

        step_buffer.clear();
        let summary = self.simulate(schedule, step_buffer)?;

        if summary.net_loss < best_solution.summary.net_loss {
            best_solution.summary = summary;
            best_solution.steps.clone_from(step_buffer);
            *n_mutations_succeeded += 1;
        } else {
            // Revert:
            schedule.insert(mutation_1.index, mutation_1.old_value);
            schedule.insert(mutation_2.index, mutation_2.old_value);
        }

        Ok(())
    }

    /// Simulate the schedule.
    fn simulate(
        &self,
        schedule: &Series<WorkingMode>,
        step_buffer: &mut Vec<(DateTime<Local>, Step)>,
    ) -> Result<Summary> {
        let min_residual_energy = self.capacity * f64::from(self.battery.min_soc_percent) / 100.0;

        let mut current_residual_energy = self.residual_energy;
        let mut net_loss = Cost::ZERO;
        let mut net_loss_without_battery = Cost::ZERO;

        for point in self.metrics.try_zip_exactly(schedule) {
            let (time, (metrics, working_mode)) = point?;
            let initial_residual_energy = current_residual_energy;

            // For missing weather forecast, assume none solar power:
            let solar_production =
                metrics.solar_power_density.unwrap_or(Quantity::ZERO) * self.pv_surface_area;
            // Positive is excess, negative is deficit:
            let power_balance = solar_production - self.consumption.stand_by;

            // Power flow to the battery (negative is directed from the battery):
            let battery_external_power = match working_mode {
                WorkingMode::Idle => Kilowatts::ZERO,
                WorkingMode::Charging => self.battery.charging_power,
                WorkingMode::Discharging => -self.battery.discharging_power,
                WorkingMode::Balancing => power_balance
                    .clamp(-self.battery.discharging_power, self.battery.charging_power),
            };

            // Power flow inside the battery corrected by the round-trip efficiency:
            let (battery_external_power, battery_active_time) = if battery_external_power
                > Kilowatts::ZERO
            {
                // While charging, the residual energy grows slower:
                let internal_power = battery_external_power * self.battery.efficiency;
                current_residual_energy = (current_residual_energy + internal_power * Hours::ONE)
                    .min(self.capacity.max(initial_residual_energy));
                let time_charging =
                    (current_residual_energy - initial_residual_energy) / internal_power;
                assert!(time_charging >= Hours::ZERO);
                (battery_external_power, time_charging)
            } else if battery_external_power < Kilowatts::ZERO {
                // While discharging, the residual energy is spent faster:
                let internal_power = battery_external_power / self.battery.efficiency;
                // Remember that the power here is negative, hence the `+`:
                current_residual_energy = (current_residual_energy + internal_power * Hours::ONE)
                    .max(min_residual_energy.min(initial_residual_energy));
                let time_discharging =
                    (current_residual_energy - initial_residual_energy) / internal_power;
                assert!(time_discharging >= Hours::ZERO);
                (battery_external_power, time_discharging)
            } else {
                // Idle:
                (Kilowatts::ZERO, Hours::ZERO)
            };

            // Self-discharging:
            current_residual_energy = (current_residual_energy
                - self.battery.self_discharge * (Hours::ONE - battery_active_time))
                .max(KilowattHours::ZERO);

            // Finally, total household energy balance:
            let production_without_battery = power_balance * Hours::ONE;
            let grid_consumption =
                battery_external_power * battery_active_time - production_without_battery;

            let loss = self.loss(metrics.grid_rate, grid_consumption);
            net_loss += loss;
            net_loss_without_battery += self.loss(metrics.grid_rate, -production_without_battery);

            step_buffer.push((
                *time,
                Step {
                    working_mode: *working_mode,
                    residual_energy_before: initial_residual_energy,
                    residual_energy_after: current_residual_energy,
                    grid_consumption,
                    loss,
                },
            ));
        }

        Ok(Summary { net_loss, net_loss_without_battery })
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
