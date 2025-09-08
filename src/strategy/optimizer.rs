use std::collections::BTreeSet;

use itertools::Itertools;

use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    prelude::*,
    strategy::{WorkingMode, simulator::Simulation},
    units::{energy::KilowattHours, power::Kilowatts, rate::EuroPerKilowattHour},
};

pub struct Optimization {
    /// Simulation result of the best solution.
    pub simulation: Simulation,

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
        pv_generation: &[Kilowatts],
        residual_energy: KilowattHours,
        capacity: KilowattHours,
        battery_args: &BatteryArgs,
        consumption_args: &ConsumptionArgs,
    ) -> Result<Self> {
        hourly_rates
            // Find all possible thresholds:
            .iter()
            .collect::<BTreeSet<_>>()
            // Iterate all possible pairs of charging-discharging thresholds:
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
                let simulation = Simulation::run(
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
                    profit = simulation.profit.to_string(),
                );
                Self { simulation, working_mode_sequence }
            })
            .max_by_key(|optimization| optimization.simulation.profit)
            .context("there is no solution")
    }
}
