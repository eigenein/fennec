use std::collections::BTreeSet;

use itertools::Itertools;

use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    prelude::*,
    strategy::{WorkingMode, simulator::Simulation},
    units::{currency::Cost, energy::KilowattHours, power::Kilowatts, rate::EuroPerKilowattHour},
};

pub struct Optimization {}

impl Optimization {
    #[instrument(
    name = "Optimising…",
    fields(residual_energy = %residual_energy),
    skip_all,
)]
    pub fn run(
        hourly_rates: &[EuroPerKilowattHour],
        pv_generation: &[Kilowatts],
        residual_energy: KilowattHours,
        capacity: KilowattHours,
        battery_args: &BatteryArgs,
        consumption_args: &ConsumptionArgs,
    ) -> Result<(Cost, Vec<WorkingMode>, Vec<KilowattHours>)> {
        // Find all possible thresholds:
        let unique_rates: Vec<_> =
            hourly_rates.iter().collect::<BTreeSet<_>>().into_iter().collect();

        // Iterate all possible pairs of charging-discharging thresholds:
        let (profit, working_mode_sequence, residual_energy_plan) = unique_rates
            .into_iter()
            .combinations_with_replacement(2)
            .map(|rates| {
                let max_charge_rate = rates[0];
                let min_discharge_rate = rates[1];
                assert!(max_charge_rate <= min_discharge_rate);

                let working_mode_sequence: Vec<WorkingMode> = hourly_rates
                    .iter()
                    .map(|hourly_rate| {
                        // TODO: introduce the «keeping» mode (force discharge with zero power)?
                        if hourly_rate <= max_charge_rate {
                            WorkingMode::Charging
                        } else if hourly_rate <= min_discharge_rate {
                            WorkingMode::Balancing
                        } else {
                            WorkingMode::Discharging
                        }
                    })
                    .collect();
                let (test_profit, residual_energy_plan) = Simulation::run(
                    hourly_rates,
                    pv_generation,
                    &working_mode_sequence,
                    residual_energy,
                    capacity,
                    battery_args,
                    consumption_args,
                );
                trace!(
                    "Simulated",
                    max_charge_rate = max_charge_rate.to_string(),
                    min_discharge_rate = min_discharge_rate.to_string(),
                    profit = test_profit.to_string(),
                );
                (test_profit, working_mode_sequence, residual_energy_plan)
            })
            .max_by_key(|(profit, _, _)| *profit)
            .context("there is no solution")?;

        // TODO: extract into a `struct` and add the thresholds there.
        Ok((profit, working_mode_sequence, residual_energy_plan))
    }
}
