use std::collections::BTreeSet;

use itertools::Itertools;
use rust_decimal::{Decimal, dec};

use crate::{
    cli::BatteryPower,
    foxess::FoxEssWorkingMode,
    nextenergy::HourlyRate,
    prelude::*,
    units::{Euro, KilowattHour, Kilowatts},
};

#[derive(derive_more::IntoIterator)]
pub struct BatteryPlan(pub Vec<BatteryPlanEntry>);

impl BatteryPlan {
    pub fn trace(&self) {
        for entry in &self.0 {
            info!(
                "Schedule",
                start_time = entry.hourly_rate.start_at.to_string(),
                mode = format!("{:?}", entry.mode),
                rate = entry.hourly_rate.energy_rate.to_string()
            );
        }
    }
}

#[derive(Copy, Clone)]
pub struct BatteryPlanEntry {
    pub mode: FoxEssWorkingMode,
    pub hourly_rate: HourlyRate,
    // TODO: should also include weather, prognosed power and residual energy.
}

#[instrument(
    name = "Optimising…",
    fields(starting_energy = %starting_energy),
    skip_all,
)]
pub fn optimise(
    hourly_rates: &[HourlyRate],
    starting_energy: KilowattHour,
    stand_by_power: Kilowatts,
    min_soc_percent: u32,
    capacity: KilowattHour,
    battery_power: BatteryPower,
) -> Result<(Euro, BatteryPlan)> {
    let min_residual_energy =
        KilowattHour(capacity.0 * Decimal::from(min_soc_percent) * dec!(0.01));

    // Find all possible thresholds:
    let unique_rates: Vec<_> = hourly_rates
        .iter()
        .map(|rate| rate.energy_rate)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    // Iterate all possible pairs of charging-discharging thresholds:
    let (profit, entries) = unique_rates
        .into_iter()
        .combinations_with_replacement(2)
        .map(|rates| {
            let max_charge_rate = rates[0];
            let min_discharge_rate = rates[1];

            let schedule: Vec<BatteryPlanEntry> = hourly_rates
                .iter()
                .map(|hourly_rate| BatteryPlanEntry {
                    hourly_rate: *hourly_rate,
                    mode: if hourly_rate.energy_rate <= max_charge_rate {
                        FoxEssWorkingMode::ForceCharge
                    } else if hourly_rate.energy_rate <= min_discharge_rate {
                        FoxEssWorkingMode::SelfUse
                    } else {
                        FoxEssWorkingMode::ForceDischarge
                    },
                })
                .collect();
            let test_profit = simulate(
                &schedule,
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
            (test_profit, schedule)
        })
        .max_by_key(|(profit, _)| *profit)
        .context("there is no solution")?;
    Ok((profit, BatteryPlan(entries)))
}

fn simulate(
    schedule: &[BatteryPlanEntry],
    starting_energy: KilowattHour,
    stand_by_power: Kilowatts,
    min_residual_energy: KilowattHour,
    capacity: KilowattHour,
    battery_power: BatteryPower,
) -> Euro {
    let mut current_energy = starting_energy;
    let mut profit = Euro(Decimal::ZERO);

    for entry in schedule {
        let power = match entry.mode {
            FoxEssWorkingMode::SelfUse => -stand_by_power,
            FoxEssWorkingMode::ForceCharge => battery_power.charging,
            FoxEssWorkingMode::ForceDischarge => -battery_power.discharging,
            FoxEssWorkingMode::Backup => unimplemented!(),
            FoxEssWorkingMode::FeedIn => unimplemented!(),
        };

        // Run the mode for 1 hour and cap it within the battery residual energy bounds:
        let new_energy =
            KilowattHour((current_energy.0 + power.0).clamp(min_residual_energy.0, capacity.0));

        // Calculate the energy change:
        let energy_change = KilowattHour(new_energy.0 - current_energy.0);
        current_energy = new_energy;

        // Calculate the associated cost:
        let cost = Euro(entry.hourly_rate.energy_rate.0 * energy_change.0);

        // Pay for charging, earn from discharging:
        profit.0 -= cost.0;
    }

    profit
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use rust_decimal::dec;

    use super::*;
    use crate::units::EuroPerKilowattHour;

    #[test]
    fn test_simulate() {
        let test_date = NaiveDate::from_ymd_opt(2025, 9, 10).unwrap();
        let schedule = [
            // +2 kWh, -2 euro:
            BatteryPlanEntry {
                mode: FoxEssWorkingMode::ForceCharge,
                hourly_rate: HourlyRate {
                    energy_rate: EuroPerKilowattHour(dec!(1.0)),
                    start_at: test_date.and_hms_opt(11, 0, 0).unwrap(),
                },
            },
            // Battery is capped at 3 kWh:
            BatteryPlanEntry {
                mode: FoxEssWorkingMode::ForceCharge,
                hourly_rate: HourlyRate {
                    energy_rate: EuroPerKilowattHour(dec!(2.0)),
                    start_at: test_date.and_hms_opt(12, 0, 0).unwrap(),
                },
            },
            // -1 kWh, +3 euro:
            BatteryPlanEntry {
                mode: FoxEssWorkingMode::SelfUse,
                hourly_rate: HourlyRate {
                    energy_rate: EuroPerKilowattHour(dec!(3.0)),
                    start_at: test_date.and_hms_opt(13, 0, 0).unwrap(),
                },
            },
            // -1 kWh, +4 euro:
            BatteryPlanEntry {
                mode: FoxEssWorkingMode::ForceDischarge,
                hourly_rate: HourlyRate {
                    energy_rate: EuroPerKilowattHour(dec!(4.0)),
                    start_at: test_date.and_hms_opt(14, 0, 0).unwrap(),
                },
            },
            // Battery is capped at 1 kWh:
            BatteryPlanEntry {
                mode: FoxEssWorkingMode::ForceDischarge,
                hourly_rate: HourlyRate {
                    energy_rate: EuroPerKilowattHour(dec!(5.0)),
                    start_at: test_date.and_hms_opt(15, 0, 0).unwrap(),
                },
            },
        ];
        let profit = simulate(
            &schedule,
            KilowattHour(dec!(1.0)), // starting at 1 kWh
            Kilowatts(dec!(1.0)),    // normally discharging at 1 kW
            KilowattHour(dec!(1.0)), // minimum at 1 kWh
            KilowattHour(dec!(3.0)), // capacity is 3 kWh
            BatteryPower { charging: Kilowatts(dec!(2.0)), discharging: Kilowatts(dec!(1.0)) },
        );
        assert_eq!(profit.0, dec!(5.0));
    }
}
