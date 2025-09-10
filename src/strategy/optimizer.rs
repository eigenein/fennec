use std::collections::BTreeSet;

use bon::Builder;
use ordered_float::OrderedFloat;

use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    prelude::*,
    strategy::{Strategy, WorkingMode},
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
}

impl Optimizer<'_> {
    #[instrument(name = "Optimising…", fields(residual_energy = %self.residual_energy), skip_all)]
    pub fn run(self) -> Result<Solution> {
        // Find all possible thresholds:
        let unique_rates = self.hourly_rates.iter().copied().collect::<BTreeSet<_>>();

        Strategy::iter_from_rates(&unique_rates)
            .map(|strategy| {
                let plan = self.simulate(strategy);
                trace!(
                    "Simulated",
                    max_charging_rate = strategy.max_charging_rate.to_string(),
                    min_discharging_rate = strategy.min_discharging_rate.to_string(),
                    profit = plan.net_profit.to_string(),
                );
                Solution { strategy, plan }
            })
            .max_by_key(|solution| solution.plan.total_profit())
            .context("there is no solution")
    }

    fn simulate(&self, strategy: Strategy) -> Plan {
        let min_residual_energy = self.capacity * f64::from(self.battery.min_soc_percent) / 100.0;

        let mut current_residual_energy = self.residual_energy;
        let mut net_profit = Cost::ZERO;
        let mut steps = Vec::with_capacity(self.hourly_rates.len());

        for (rate, solar_power) in
            self.hourly_rates.iter().copied().zip(self.solar_power.iter().copied())
        {
            // Positive, when solar power is greater than the stand-by consumption,
            // and negative, otherwise.
            let power_balance = solar_power - self.consumption.stand_by;

            // Theoretical charging rate, euro per kilowatt-hour:
            let charging_rate = {
                let solar_charging = (solar_power - self.consumption.stand_by)
                    .clamp(Kilowatts::ZERO, self.battery.charging_power);
                let grid_charging = self.battery.charging_power - solar_charging;

                // Grid import is charged with the full rate:
                grid_charging * rate
                    // Excessive solar power has lower value:
                    + solar_charging * (rate - self.consumption.purchase_fees)
            } / self.battery.charging_power;
            let is_charging_allowed = charging_rate <= strategy.max_charging_rate;

            // Theoretical discharging rate, euro per kilowatt-hour:
            let discharging_rate = {
                let internal_consumption =
                    self.battery.discharging_power.min(self.consumption.stand_by);
                let grid_export = self.battery.discharging_power - internal_consumption;
                assert_eq!(internal_consumption + grid_export, self.battery.discharging_power);

                // Internal consumption compensates the full rate:
                internal_consumption * rate
                    // Whilst grid export is less beneficial:
                    + grid_export * (rate - self.consumption.purchase_fees)
            } / self.battery.discharging_power;
            let is_discharging_allowed = discharging_rate >= strategy.min_discharging_rate;

            // Figure out the working mode and effective power:
            let (working_mode, power) = if is_charging_allowed {
                if is_discharging_allowed {
                    // «Self-use», so covering the deficit and accumulating the excess:
                    (WorkingMode::Balancing, power_balance)
                } else {
                    // Forcibly charging:
                    (WorkingMode::Charging, self.battery.charging_power)
                }
            } else if is_discharging_allowed {
                // Forcibly discharging:
                (WorkingMode::Discharging, -self.battery.discharging_power)
            } else {
                // Just keep fingers crossed:
                (WorkingMode::Maintain, Kilowatts::ZERO)
            };

            // Update the residual energy:
            let residual_energy_before = current_residual_energy;
            current_residual_energy = if power > Kilowatts::ZERO {
                // The actual residual energy grows slower:
                (residual_energy_before + power * Hours::ONE * self.battery.efficiency)
                    .min(self.capacity)
            } else {
                // The residual energy is spent faster:
                (residual_energy_before + power * Hours::ONE / self.battery.efficiency)
                    .max(min_residual_energy)
            };

            // And the step profit:
            let profit = if power > Kilowatts::ZERO {
                assert!(residual_energy_before <= current_residual_energy);
                (residual_energy_before - current_residual_energy) * charging_rate
            } else {
                assert!(residual_energy_before >= current_residual_energy);
                (residual_energy_before - current_residual_energy) * discharging_rate
            };

            steps.push(HourStep {
                working_mode,
                residual_energy_before,
                residual_energy_after: current_residual_energy,
                profit,
                effective_charging_rate: charging_rate,
                effective_discharging_rate: discharging_rate,
            });
            net_profit += profit;
        }

        let residual_energy_value = {
            let usable_residual_energy =
                steps.last().unwrap().residual_energy_after - min_residual_energy;
            #[allow(clippy::cast_precision_loss)]
            let average_buying_rate = self.hourly_rates.iter().copied().sum::<KilowattHourRate>()
                / self.hourly_rates.len() as f64;
            let average_selling_rate = average_buying_rate - self.consumption.purchase_fees;
            if usable_residual_energy >= KilowattHours::ZERO {
                // Theoretical money we can make from selling it all at once:
                usable_residual_energy * self.battery.efficiency * average_selling_rate
            } else {
                // Uh-oh, we need to spend at least this much money to compensate the self-discharge:
                usable_residual_energy / self.battery.efficiency * average_buying_rate
            }
        };

        Plan { net_profit, residual_energy_value, steps }
    }
}

/// Optimization plan that describes how the battery will work in the upcoming hours.
pub struct Plan {
    /// Calculated profit.
    pub net_profit: Cost,

    /// Minimal selling cost of the residual energy by the end of the simulation.
    ///
    /// It may be negative, that would mean losses due to the self-discharge below the minimal SoC.
    #[expect(clippy::doc_markdown)]
    pub residual_energy_value: Cost,

    /// Hourly forecast.
    pub steps: Vec<HourStep>,
}

impl Plan {
    /// Sum of the net profit and the residual energy value.
    pub fn total_profit(&self) -> Cost {
        self.net_profit + self.residual_energy_value
    }
}

/// Single-hour working plan step.
pub struct HourStep {
    pub effective_charging_rate: KilowattHourRate,
    pub effective_discharging_rate: KilowattHourRate,
    pub working_mode: WorkingMode,
    pub residual_energy_before: KilowattHours,
    pub residual_energy_after: KilowattHours,
    pub profit: Cost,
}

pub struct Solution {
    pub strategy: Strategy,
    pub plan: Plan,
}
