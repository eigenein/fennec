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
    units::{energy::KilowattHours, power::Kilowatts, rate::KilowattHourRate},
};

pub struct Solution {
    pub outcome: Outcome,
    pub strategy: Strategy,
    pub working_mode_sequence: Vec<WorkingMode>,
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

        // Allow the thresholds to settle below or above the actual rates:
        unique_rates.insert(minimal_buying_rate - KilowattHourRate(dec!(0.01)));
        unique_rates
            .insert(*unique_rates.iter().next_back().unwrap() + KilowattHourRate(dec!(0.01)));

        Strategy::iter_from_rates(&unique_rates)
            .map(|strategy| {
                let working_mode_sequence: Vec<WorkingMode> = self
                    .hourly_rates
                    .iter()
                    .copied()
                    .map(|hourly_rate| {
                        if let Some(max_charging_rate) = strategy.max_charging_rate
                            && hourly_rate <= max_charging_rate
                        {
                            WorkingMode::Charging
                        } else if let Some(min_discharging_rate) = strategy.min_discharging_rate
                            && hourly_rate >= min_discharging_rate
                        {
                            WorkingMode::Discharging
                        } else {
                            WorkingMode::Balancing
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
                trace!(
                    "Simulated",
                    max_charging_rate = format!("{:?}", strategy.max_charging_rate),
                    min_discharging_rate = format!("{:?}", strategy.min_discharging_rate),
                    profit = outcome.net_profit.to_string(),
                );
                Solution { outcome, strategy, working_mode_sequence }
            })
            .max_by_key(|solution| solution.outcome.total_profit())
            .context("there is no solution")
    }
}
