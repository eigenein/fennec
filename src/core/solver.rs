use std::{collections::HashMap, ops::Index};

use bon::{Builder, builder};
use chrono::{DateTime, Local, Timelike};

use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    core::{metrics::Metrics, series::Series, step::Step, working_mode::WorkingMode},
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
#[builder(finish_fn(vis = ""))]
pub struct Solver<'a> {
    metrics: &'a Series<Metrics>,
    pv_surface_area: SquareMetres,
    residual_energy: KilowattHours,
    capacity: KilowattHours,
    battery: BatteryArgs,
    consumption: ConsumptionArgs,
    stand_by_power: [Option<Kilowatts>; 24],
}

impl<'s, S: solver_builder::IsComplete> SolverBuilder<'s, S> {
    pub fn solve(self) {
        self.build().solve()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct Input {
    /// Index in the metrics series.
    index: usize,

    /// Discretized residual energy in watt-hours.
    residual_energy_wh: u32,
}

impl<'a> Solver<'a> {
    /// Find the optimal battery schedule.
    fn solve(self) {
        todo!()
    }

    /// Simulate the battery working in the specified mode given the initial conditions,
    /// and return the loss and new residual energy.
    fn step(
        &self,
        metrics_index: usize,
        initial_residual_energy: KilowattHours,
        min_residual_energy: KilowattHours,
        working_mode: WorkingMode,
    ) -> (Cost, KilowattHours) {
        let start_time = *self.metrics.index_at(metrics_index);
        let metrics = &self.metrics[metrics_index];

        let mut current_residual_energy = initial_residual_energy;
        let stand_by_power =
            self.stand_by_power[start_time.hour() as usize].unwrap_or(self.consumption.stand_by);

        // For missing weather forecast, assume none solar power:
        let solar_production =
            metrics.solar_power_density.unwrap_or(Quantity::ZERO) * self.pv_surface_area;
        // Positive is excess, negative is deficit:
        let power_balance = solar_production - stand_by_power;

        // Power flow to the battery (negative is directed from the battery):
        let battery_external_power = match working_mode {
            WorkingMode::Idle => Kilowatts::ZERO,
            WorkingMode::Charging => self.battery.charging_power,
            WorkingMode::Discharging => -self.battery.discharging_power,
            WorkingMode::Balancing => {
                power_balance.clamp(-self.battery.discharging_power, self.battery.charging_power)
            }
        };

        // Power flow inside the battery corrected by the round-trip efficiency:
        let (battery_external_power, battery_active_time) =
            if battery_external_power > Kilowatts::ZERO {
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
        (loss, current_residual_energy)
    }

    /// Calculate the grid consumption or production loss.
    fn loss(&self, grid_rate: KilowattHourRate, consumption: KilowattHours) -> Cost {
        if consumption >= KilowattHours::ZERO {
            consumption * grid_rate
        } else {
            // We sell excess energy cheaper:
            consumption * (grid_rate - self.consumption.purchase_fees)
        }
    }
}
