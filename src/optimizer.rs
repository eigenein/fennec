use std::collections::BTreeSet;

use itertools::Itertools;
use rust_decimal::Decimal;

pub use self::working_mode::{WorkingMode, WorkingModeHourlySchedule};
use crate::{
    cli::HuntArgs,
    prelude::*,
    units::{Euro, EuroPerKilowattHour, KilowattHours},
};

mod working_mode;

#[instrument(
    name = "Optimising…",
    fields(residual_energy = %residual_energy),
    skip_all,
)]
pub fn optimise(
    hourly_rates: &[EuroPerKilowattHour],
    residual_energy: KilowattHours,
    capacity: KilowattHours,
    hunt_args: &HuntArgs,
) -> Result<(Euro, Vec<WorkingMode>)> {
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
                        WorkingMode::Balancing
                    } else {
                        WorkingMode::Discharging
                    }
                })
                .collect();
            let test_profit = simulate(
                hourly_rates,
                &working_mode_sequence,
                residual_energy,
                capacity,
                hunt_args,
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
    residual_energy: KilowattHours,
    capacity: KilowattHours,
    hunt_args: &HuntArgs,
) -> Euro {
    let min_residual_energy = KilowattHours(
        capacity.0 * Decimal::from(hunt_args.battery.min_soc_percent) / Decimal::ONE_HUNDRED,
    );
    let mut current_residual_energy = residual_energy;
    let mut profit = Euro(Decimal::ZERO);

    for (rate, working_mode) in hourly_rates.iter().zip(working_mode_sequence.as_ref()) {
        let (power, rate) = match working_mode {
            WorkingMode::Balancing => (-hunt_args.stand_by_power, *rate), // TODO: add solar forecast.
            WorkingMode::Charging => (hunt_args.battery.charging_power, *rate),
            WorkingMode::Discharging => {
                // We don't get the purchase fees back when feeding out:
                (-hunt_args.battery.discharging_power, *rate - hunt_args.purchase_fees)
            }
        };

        // Run the mode for 1 hour and cap it within the battery residual energy bounds:
        let (new_residual_energy, billable_energy) = if power.0.is_sign_negative() {
            // Discharging: we lose the residual energy faster.
            let new_residual_energy = KilowattHours(
                (current_residual_energy.0 + power.0 / hunt_args.battery.round_trip_efficiency)
                    .max(min_residual_energy.0),
            );
            (
                new_residual_energy,
                KilowattHours(
                    // But our actual billable output is lower:
                    (new_residual_energy - current_residual_energy).0
                        * hunt_args.battery.round_trip_efficiency,
                ),
            )
        } else {
            // Charging: we charge slower.
            let new_residual_energy = KilowattHours(
                (current_residual_energy.0 + power.0 * hunt_args.battery.round_trip_efficiency)
                    .min(capacity.0),
            );
            (
                new_residual_energy,
                KilowattHours(
                    // But we get billed for the full power:
                    (new_residual_energy - current_residual_energy).0
                        / hunt_args.battery.round_trip_efficiency,
                ),
            )
        };

        // Update the simulated residual energy and correct for the self-discharge loss:
        current_residual_energy = new_residual_energy
            - KilowattHours(current_residual_energy.0 * hunt_args.battery.self_discharging_rate);

        // Calculate the associated cost:
        let cost = Euro(rate.0 * billable_energy.0);

        // Pay for charging, earn from discharging:
        profit.0 -= cost.0;
    }

    profit
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;
    use crate::{
        cli::BatteryArgs,
        units::{EuroPerKilowattHour, Kilowatts},
    };

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
            WorkingMode::Charging,    // +3 kWh, -3 euro
            WorkingMode::Charging,    // battery is capped at 4 kWh
            WorkingMode::Balancing,   // -1 kWh, +3 euro
            WorkingMode::Discharging, //-2 kWh, +8 euro
            WorkingMode::Discharging, // battery is capped at 1 kWh
        ];
        let profit = simulate(
            &rates,
            &working_mode_sequence,
            KilowattHours(dec!(1.0)), // starting at 1 kWh
            KilowattHours(dec!(4.0)), // capacity is 4 kWh
            &HuntArgs {
                scout: true,
                battery: BatteryArgs {
                    charging_power: Kilowatts(dec!(3.0)),
                    discharging_power: Kilowatts(dec!(2.0)),
                    round_trip_efficiency: Decimal::ONE,
                    self_discharging_rate: Decimal::ZERO,
                    min_soc_percent: 25, // 1 kWh
                },
                stand_by_power: Kilowatts(Decimal::ONE),
                purchase_fees: EuroPerKilowattHour(Decimal::ZERO),
            },
        );
        assert_eq!(profit.0, dec!(8.0));
    }
}
