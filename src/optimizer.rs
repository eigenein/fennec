pub mod working_mode;

use std::collections::BTreeSet;

use itertools::Itertools;
use rust_decimal::{Decimal, dec};

use crate::{
    cli::BatteryPower,
    optimizer::working_mode::WorkingMode,
    prelude::*,
    units::{Euro, EuroPerKilowattHour, KilowattHour, Kilowatts},
};

#[instrument(
    name = "Optimising…",
    fields(starting_energy = %starting_energy),
    skip_all,
)]
pub fn optimise(
    hourly_rates: &[EuroPerKilowattHour],
    starting_energy: KilowattHour,
    stand_by_power: Kilowatts,
    min_soc_percent: u32,
    capacity: KilowattHour,
    battery_power: BatteryPower,
) -> Result<(Euro, Vec<WorkingMode>)> {
    let min_residual_energy =
        KilowattHour(capacity.0 * Decimal::from(min_soc_percent) * dec!(0.01));

    // Find all possible thresholds:
    let unique_rates: Vec<_> = hourly_rates.iter().collect::<BTreeSet<_>>().into_iter().collect();

    // Iterate all possible pairs of charging-discharging thresholds:
    let (profit, working_mode_sequence) = unique_rates
        .into_iter()
        .combinations_with_replacement(2)
        .map(|rates| {
            let max_charge_rate = rates[0];
            let min_discharge_rate = rates[1];

            let working_mode_sequence: Vec<WorkingMode> = hourly_rates
                .iter()
                .map(|hourly_rate| {
                    if hourly_rate <= max_charge_rate {
                        WorkingMode::Charging
                    } else if hourly_rate <= min_discharge_rate {
                        WorkingMode::SelfUse
                    } else {
                        WorkingMode::Discharging
                    }
                })
                .collect();
            let test_profit = simulate(
                hourly_rates,
                &working_mode_sequence,
                starting_energy,
                stand_by_power,
                min_residual_energy,
                capacity,
                battery_power,
            );
            trace!(
                "Simulated",
                max_charge_rate = max_charge_rate.to_string(),
                min_discharge_rate = min_discharge_rate.to_string(),
                profit = test_profit.to_string(),
            );
            (test_profit, working_mode_sequence)
        })
        .max_by_key(|(profit, _)| *profit)
        .context("there is no solution")?;
    Ok((profit, working_mode_sequence))
}

fn simulate(
    hourly_rates: &[EuroPerKilowattHour],
    working_mode_sequence: &[WorkingMode],
    starting_energy: KilowattHour,
    stand_by_power: Kilowatts,
    min_residual_energy: KilowattHour,
    capacity: KilowattHour,
    battery_power: BatteryPower,
) -> Euro {
    let mut current_energy = starting_energy;
    let mut profit = Euro(Decimal::ZERO);

    for (rate, working_mode) in hourly_rates.iter().zip(working_mode_sequence.as_ref()) {
        let power = match working_mode {
            WorkingMode::SelfUse => -stand_by_power,
            WorkingMode::Charging => battery_power.charging,
            WorkingMode::Discharging => -battery_power.discharging,
        };

        // Run the mode for 1 hour and cap it within the battery residual energy bounds:
        let new_energy =
            KilowattHour((current_energy.0 + power.0).clamp(min_residual_energy.0, capacity.0));

        // Calculate the energy change:
        let energy_change = KilowattHour(new_energy.0 - current_energy.0);
        current_energy = new_energy;

        // Calculate the associated cost:
        let cost = Euro(rate.0 * energy_change.0);

        // Pay for charging, earn from discharging:
        profit.0 -= cost.0;
    }

    profit
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;
    use crate::units::EuroPerKilowattHour;

    #[test]
    fn test_simulate() {
        let rates = [
            EuroPerKilowattHour(dec!(1.0)),
            EuroPerKilowattHour(dec!(2.0)),
            EuroPerKilowattHour(dec!(3.0)),
            EuroPerKilowattHour(dec!(4.0)),
            EuroPerKilowattHour(dec!(5.0)),
        ];
        let working_mode_sequence = [
            WorkingMode::Charging,    // +2 kWh, -2 euro
            WorkingMode::Charging,    // battery is capped at 3 kWh
            WorkingMode::SelfUse,     // -1 kWh, +3 euro
            WorkingMode::Discharging, //-1 kWh, +4 euro
            WorkingMode::Discharging, // battery is capped at 1 kWh
        ];
        let profit = simulate(
            &rates,
            &working_mode_sequence,
            KilowattHour(dec!(1.0)), // starting at 1 kWh
            Kilowatts(dec!(1.0)),    // normally discharging at 1 kW
            KilowattHour(dec!(1.0)), // minimum at 1 kWh
            KilowattHour(dec!(3.0)), // capacity is 3 kWh
            BatteryPower { charging: Kilowatts(dec!(2.0)), discharging: Kilowatts(dec!(1.0)) },
        );
        assert_eq!(profit.0, dec!(5.0));
    }
}
