use std::collections::BTreeSet;

use bon::Builder;
use rust_decimal::dec;

use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    prelude::*,
    strategy::{
        Strategy,
        WorkingMode,
        simulator::{Outcome, Simulator},
    },
    units::{currency::Cost, energy::KilowattHours, power::Kilowatts, rate::KilowattHourRate},
};

pub struct Solution {
    pub outcome: Outcome,
    pub strategy: Strategy,
    pub working_mode_sequence: Vec<WorkingMode>,

    #[deprecated = "this should go to `Solution`"]
    pub minimal_residual_energy_value: Cost,
}

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
        let mut unique_rates = self.hourly_rates.iter().copied().collect::<BTreeSet<_>>();
        let minimal_buying_rate = *unique_rates.iter().next().unwrap();

        // I'll use the minimal rate to estimate the residual energy value.
        let minimal_selling_rate = minimal_buying_rate - self.consumption.purchase_fees;
        let min_residual_energy = self.capacity * f64::from(self.battery.min_soc_percent) / 100.0;

        // Allow the thresholds to settle below or above the actual rates:
        unique_rates.insert(minimal_buying_rate - KilowattHourRate(dec!(0.01)));
        unique_rates
            .insert(*unique_rates.iter().next_back().unwrap() + KilowattHourRate(dec!(0.01)));

        Strategy::iter_from_rates(unique_rates)
            .map(|strategy| {
                let working_mode_sequence: Vec<WorkingMode> = self
                    .hourly_rates
                    .iter()
                    .copied()
                    .map(|hourly_rate| {
                        // TODO: introduce the «keeping» mode (force discharge with zero power)?
                        if hourly_rate <= strategy.max_charging_rate {
                            WorkingMode::Charging
                        } else if hourly_rate <= strategy.min_discharging_rate {
                            WorkingMode::Balancing
                        } else {
                            WorkingMode::Discharging
                        }
                    })
                    .collect();
                let outcome = Simulator::builder()
                    .hourly_rates(self.hourly_rates)
                    .solar_power(self.solar_power)
                    .working_mode_sequence(&working_mode_sequence)
                    .residual_energy(self.residual_energy)
                    .capacity(self.capacity)
                    .battery(self.battery)
                    .consumption(self.consumption)
                    .build()
                    .run();
                let usable_residual_energy =
                    outcome.forecast.last().unwrap().residual_energy_after - min_residual_energy;
                let minimal_residual_energy_value = if usable_residual_energy.is_non_negative() {
                    // Theoretical money we can make from selling it all at once:
                    usable_residual_energy
                        * self.battery.discharging_efficiency
                        * minimal_selling_rate
                } else {
                    // Uh-oh, we need to spend money at least this much money to compensate the self-discharge:
                    usable_residual_energy / self.battery.charging_efficiency * minimal_buying_rate
                };
                trace!(
                    "Simulated",
                    max_charging_rate = strategy.max_charging_rate.to_string(),
                    min_discharging_rate = strategy.min_discharging_rate.to_string(),
                    profit = outcome.net_profit.to_string(),
                );
                Solution { outcome, strategy, working_mode_sequence, minimal_residual_energy_value }
            })
            .max_by_key(|optimization| {
                optimization.outcome.net_profit + optimization.minimal_residual_energy_value
            })
            .context("there is no solution")
    }
}
