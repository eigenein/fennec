use std::collections::BTreeSet;

use itertools::Itertools;
use rust_decimal::dec;

use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    prelude::*,
    strategy::{WorkingMode, simulator::Simulation},
    units::{energy::KilowattHours, power::Kilowatts, rate::EuroPerKilowattHour},
};

pub struct Optimization {
    /// Simulation result of the best solution.
    pub simulation: Simulation,

    pub max_charge_rate: EuroPerKilowattHour,

    pub min_discharge_rate: EuroPerKilowattHour,

    pub working_mode_sequence: Vec<WorkingMode>,
}

impl Optimization {
    #[instrument(
    name = "Optimising…",
    fields(residual_energy = %residual_energy),
    skip_all,
)]
    pub fn run(
        hourly_rates: &[EuroPerKilowattHour],
        solar_energy: &[Kilowatts],
        residual_energy: KilowattHours,
        capacity: KilowattHours,
        battery_args: &BatteryArgs,
        consumption_args: &ConsumptionArgs,
    ) -> Result<Self> {
        // Find all possible thresholds:
        let mut unique_rates = hourly_rates.iter().copied().collect::<BTreeSet<_>>();

        // Allow the thresholds to settle below or above the actual rates:
        unique_rates.insert(*unique_rates.iter().next().unwrap() - EuroPerKilowattHour(dec!(0.01)));
        unique_rates
            .insert(*unique_rates.iter().next_back().unwrap() + EuroPerKilowattHour(dec!(0.01)));

        unique_rates
            // Iterate all possible pairs of charging-discharging thresholds:
            .into_iter()
            .combinations_with_replacement(2)
            .map(|rates| {
                let max_charge_rate = rates[0];
                let min_discharge_rate = rates[1];
                assert!(max_charge_rate <= min_discharge_rate);

                let working_mode_sequence: Vec<WorkingMode> = hourly_rates
                    .iter()
                    .copied()
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
                let simulation = Simulation::run(
                    hourly_rates,
                    solar_energy,
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
                    profit = simulation.net_profit.to_string(),
                );
                Self { simulation, max_charge_rate, min_discharge_rate, working_mode_sequence }
            })
            .max_by_key(|optimization| optimization.simulation.net_profit)
            .context("there is no solution")
    }
}
