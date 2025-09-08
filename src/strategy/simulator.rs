use chrono::TimeDelta;

use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    strategy::WorkingMode,
    units::{currency::Cost, energy::KilowattHours, power::Kilowatts, rate::EuroPerKilowattHour},
};

pub struct Simulation {
    /// Calculated profit.
    pub net_profit: Cost,

    /// Hourly forecast.
    pub forecast: Vec<Forecast>,
}

pub struct Forecast {
    pub residual_energy_before: KilowattHours,
    pub residual_energy_after: KilowattHours,
    pub grid_energy_used: KilowattHours,
    pub net_profit: Cost,
}

impl Simulation {
    pub fn run(
        hourly_rates: &[EuroPerKilowattHour],
        solar_energy: &[Kilowatts],
        working_mode_sequence: &[WorkingMode],
        residual_energy: KilowattHours,
        capacity: KilowattHours,
        battery_args: &BatteryArgs,
        consumption_args: &ConsumptionArgs,
    ) -> Self {
        const ONE_HOUR: TimeDelta = TimeDelta::hours(1);
        let min_residual_energy = capacity * f64::from(battery_args.min_soc_percent) / 100.0;

        let mut current_residual_energy = residual_energy;
        let mut net_profit = Cost::ZERO;
        let mut forecast = Vec::with_capacity(hourly_rates.len());

        for ((rate, working_mode), solar_power) in
            hourly_rates.iter().zip(working_mode_sequence.as_ref()).zip(solar_energy)
        {
            let residual_energy_before = current_residual_energy;

            // Here's what's happening at the battery connection point:
            let power_balance = match working_mode {
                WorkingMode::Charging => battery_args.charging_power,
                WorkingMode::Discharging => battery_args.discharging_power,
                WorkingMode::Balancing => *solar_power + consumption_args.stand_by_power,
            };

            // Charging:
            if power_balance.0.is_sign_positive() {
                // Let's see how much energy is spent charging it taking the power balance and capacity into account:
                let billable_energy_differential = (capacity - current_residual_energy)
                    .min(battery_args.charging_power.min(power_balance) * ONE_HOUR);
                assert!(billable_energy_differential.is_non_negative());

                // Calculate the distribution between the available grid and PV energy:
                let pv_energy_used = (*solar_power * ONE_HOUR).min(billable_energy_differential);
                let grid_energy_used = billable_energy_differential - pv_energy_used;
                assert!(pv_energy_used.is_non_negative());
                assert!(grid_energy_used.is_non_negative());

                // Calculate the associated costs:
                let hour_net_profit =
                    // For PV energy, we estimate the lost profit without the purchase fees:
                    -pv_energy_used * (*rate - consumption_args.purchase_fees)
                    // For grid energy, we are buying it at the full rate:
                    - grid_energy_used * *rate;
                net_profit += hour_net_profit;

                // Update current residual energy taking the efficiency into account:
                current_residual_energy +=
                    billable_energy_differential * battery_args.charging_efficiency;

                forecast.push(Forecast {
                    residual_energy_after: current_residual_energy,
                    grid_energy_used,
                    residual_energy_before,
                    net_profit: hour_net_profit,
                });
            }
            // Discharging:
            else if power_balance.0.is_sign_negative() {
                // Pre-apply self-discharging (to get the average between the initial and resulting residual energy):
                current_residual_energy -=
                    current_residual_energy * battery_args.self_discharging_rate * 0.5;
                assert!(current_residual_energy.is_non_negative());

                // Let's see how much energy we can obtain taking the minimum SoC and power balance into account.
                let internal_energy_differential =
                    // Usable residual energy:
                    (min_residual_energy - current_residual_energy)
                    .clamp(
                        // Limited by actual discharging power corrected by the efficiency:
                    battery_args.discharging_power.max(power_balance) / battery_args.discharging_efficiency * ONE_HOUR,
                        // The self-discharging could already drop the residual energy below the reserve:
                        KilowattHours::ZERO,
                    );
                assert!(internal_energy_differential.is_non_positive());

                // But, we actually get less from it due to the efficiency losses:
                let billable_energy_differential =
                    internal_energy_differential * battery_args.discharging_efficiency;
                assert!(billable_energy_differential.is_non_positive());

                // Calculate the payback (`max` is because the differential is negative):
                let stand_by_differential =
                    billable_energy_differential.max(consumption_args.stand_by_power * ONE_HOUR);
                let grid_differential = billable_energy_differential - stand_by_differential;
                assert!(stand_by_differential.is_non_positive());
                assert!(
                    grid_differential.is_non_positive(),
                    "grid differential: {grid_differential}",
                );
                let hour_net_profit =
                    // Equivalent stand-by consumption from the grid would be billed with the full rate:
                    -stand_by_differential * *rate
                    // The rest we sell a little cheaper, without the purchase fees:
                    - grid_differential * (*rate - consumption_args.purchase_fees);
                net_profit += hour_net_profit;

                // Update current residual energy:
                current_residual_energy += internal_energy_differential;

                // Post-apply self-discharging:
                current_residual_energy -=
                    current_residual_energy * battery_args.self_discharging_rate * 0.5;
                assert!(current_residual_energy.is_non_negative());

                forecast.push(Forecast {
                    residual_energy_after: current_residual_energy,
                    grid_energy_used: grid_differential,
                    residual_energy_before,
                    net_profit: hour_net_profit,
                });
            }
        }

        Self { net_profit, forecast }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;
    use crate::cli::BatteryArgs;

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
        let solar_energy = [Kilowatts(0.0); 5];
        let simulation = Simulation::run(
            &rates,
            &solar_energy,
            &working_mode_sequence,
            KilowattHours(1.0), // starting at 1 kWh
            KilowattHours(4.0), // capacity is 4 kWh
            &BatteryArgs {
                charging_power: Kilowatts(3.0),
                discharging_power: Kilowatts(-2.0),
                charging_efficiency: 1.0,
                discharging_efficiency: 1.0,
                self_discharging_rate: 0.0,
                min_soc_percent: 25, // 1 kWh
            },
            &ConsumptionArgs {
                stand_by_power: -Kilowatts(1.0),
                purchase_fees: EuroPerKilowattHour(dec!(0.0)),
            },
        );
        assert_eq!(simulation.net_profit.0, 8.0);
    }
}
