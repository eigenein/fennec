use chrono::TimeDelta;

use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    strategy::WorkingMode,
    units::{currency::Cost, energy::KilowattHours, power::Kilowatts, rate::EuroPerKilowattHour},
};

pub struct Simulation {
    /// Calculated profit.
    pub profit: Cost,

    pub residual_energy_forecast: Vec<KilowattHours>,
}

impl Simulation {
    pub fn run(
        hourly_rates: &[EuroPerKilowattHour],
        pv_generation: &[Kilowatts],
        working_mode_sequence: &[WorkingMode],
        residual_energy: KilowattHours,
        capacity: KilowattHours,
        battery_args: &BatteryArgs,
        consumption_args: &ConsumptionArgs,
    ) -> Self {
        const ONE_HOUR: TimeDelta = TimeDelta::hours(1);
        let min_residual_energy = capacity * f64::from(battery_args.min_soc_percent) / 100.0;

        let mut current_residual_energy = residual_energy;
        let mut profit = Cost::ZERO;
        let mut residual_energy_forecast = Vec::with_capacity(hourly_rates.len());

        for ((rate, working_mode), pv_power) in
            hourly_rates.iter().zip(working_mode_sequence.as_ref()).zip(pv_generation)
        {
            // Here's what's happening at the battery connection point:
            let power_balance = match working_mode {
                WorkingMode::Charging => battery_args.charging_power,
                WorkingMode::Discharging => battery_args.discharging_power,
                WorkingMode::Balancing => *pv_power + consumption_args.stand_by_power,
            };

            // Charging:
            if power_balance.0.is_sign_positive() {
                // Let's see how much energy is spent charging it taking the power balance and capacity into account:
                let billable_energy_differential = (capacity - current_residual_energy)
                    .min(battery_args.charging_power.min(power_balance) * ONE_HOUR);
                assert!(billable_energy_differential.is_non_negative());

                // Calculate the distribution between the available grid and PV energy:
                let pv_energy_used = (*pv_power * ONE_HOUR).min(billable_energy_differential);
                let grid_energy_used = billable_energy_differential - pv_energy_used;
                assert!(pv_energy_used.is_non_negative());
                assert!(grid_energy_used.is_non_negative());

                // Calculate the associated costs:
                profit -=
                // For PV energy, we estimate the lost profit without the purchase fees:
                pv_energy_used * (*rate - consumption_args.purchase_fees)
                // For grid energy, we are buying it at the full rate:
                + grid_energy_used * *rate;

                // Update current residual energy taking the efficiency into account:
                current_residual_energy +=
                    billable_energy_differential * battery_args.charging_efficiency;
            }
            // Discharging:
            else if power_balance.0.is_sign_negative() {
                // Pre-apply self-discharging (to get the average between the initial and resulting residual energy):
                current_residual_energy -=
                    current_residual_energy * battery_args.self_discharging_rate * 0.5;
                assert!(current_residual_energy.is_non_negative());

                // Let's see how much energy we can obtain taking the minimum SoC and power balance into account.
                // I'm clamping to zero because the self-discharging could drop the residual energy below the reserve:
                let energy_differential = (min_residual_energy - current_residual_energy).clamp(
                    battery_args.discharging_power.max(power_balance) * ONE_HOUR,
                    KilowattHours::ZERO,
                );
                assert!(
                    energy_differential.is_non_positive(),
                    "energy differential: {energy_differential}",
                );

                // But, we actually get less from it due to the efficiency losses:
                let billable_energy_differential =
                    energy_differential * battery_args.discharging_efficiency;
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
                profit -=
                // Equivalent stand-by consumption from the grid would be billed with the full rate:
                stand_by_differential * *rate
                // The rest we sell a little cheaper, without the purchase fees:
                + grid_differential * (*rate - consumption_args.purchase_fees);

                // Update current residual energy:
                current_residual_energy += energy_differential;

                // Post-apply self-discharging:
                current_residual_energy -=
                    current_residual_energy * battery_args.self_discharging_rate * 0.5;
                assert!(current_residual_energy.is_non_negative());
            }

            residual_energy_forecast.push(current_residual_energy);
        }

        Self { profit, residual_energy_forecast }
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
        let pv_generation = [Kilowatts(0.0); 5];
        let simulation = Simulation::run(
            &rates,
            &pv_generation,
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
        assert_eq!(simulation.profit.0, 8.0);
    }
}
