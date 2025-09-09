use bon::Builder;
use chrono::TimeDelta;

use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    strategy::WorkingMode,
    units::{currency::Cost, energy::KilowattHours, power::Kilowatts, rate::KilowattHourRate},
};

/// Simulation outcome.
pub struct Outcome {
    /// Calculated profit.
    pub net_profit: Cost,

    /// Minimal selling cost of the residual energy by the end of the simulation.
    ///
    /// It may be negative, that would mean losses due to the self-discharge below the minimal SoC.
    #[expect(clippy::doc_markdown)]
    pub minimal_residual_energy_value: Cost,

    /// Hourly forecast.
    pub forecast: Vec<Forecast>,
}

pub struct Forecast {
    pub residual_energy_before: KilowattHours,
    pub residual_energy_after: KilowattHours,
    pub grid_energy_used: KilowattHours,
    pub net_profit: Cost,
}

#[derive(Builder)]
pub struct Simulator<'a> {
    hourly_rates: &'a [KilowattHourRate],
    solar_power: &'a [Kilowatts],
    working_mode_sequence: &'a [WorkingMode],
    residual_energy: KilowattHours,
    capacity: KilowattHours,
    battery: &'a BatteryArgs,
    consumption: &'a ConsumptionArgs,
}

impl Simulator<'_> {
    pub fn run(self) -> Outcome {
        const ONE_HOUR: TimeDelta = TimeDelta::hours(1);
        let min_residual_energy = self.capacity * f64::from(self.battery.min_soc_percent) / 100.0;

        let mut current_residual_energy = self.residual_energy;
        let mut net_profit = Cost::ZERO;
        let mut forecast = Vec::with_capacity(self.hourly_rates.len());

        for ((rate, working_mode), solar_power) in
            self.hourly_rates.iter().zip(self.working_mode_sequence.as_ref()).zip(self.solar_power)
        {
            let residual_energy_before = current_residual_energy;

            // Here's what's happening at the battery connection point:
            let power_balance = match working_mode {
                WorkingMode::Charging => self.battery.charging_power,
                WorkingMode::Discharging => self.battery.discharging_power,
                WorkingMode::Balancing => *solar_power + self.consumption.stand_by_power,
            };

            // Charging:
            if power_balance.0.is_sign_positive() {
                // Let's see how much energy is spent charging it taking the power balance and capacity into account:
                let billable_energy_differential = (self.capacity - current_residual_energy)
                    .min(self.battery.charging_power.min(power_balance) * ONE_HOUR);
                assert!(billable_energy_differential.is_non_negative());

                // Calculate the distribution between the available grid and PV energy:
                let pv_energy_used = (*solar_power * ONE_HOUR).min(billable_energy_differential);
                let grid_energy_used = billable_energy_differential - pv_energy_used;
                assert!(pv_energy_used.is_non_negative());
                assert!(grid_energy_used.is_non_negative());

                // Calculate the associated costs:
                let hour_net_profit =
                    // For PV energy, we estimate the lost profit without the purchase fees:
                    -pv_energy_used * (*rate - self.consumption.purchase_fees)
                    // For grid energy, we are buying it at the full rate:
                    - grid_energy_used * *rate;
                net_profit += hour_net_profit;

                // Update current residual energy taking the efficiency into account:
                current_residual_energy +=
                    billable_energy_differential * self.battery.charging_efficiency;

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
                    current_residual_energy * self.battery.self_discharging_rate * 0.5;
                assert!(current_residual_energy.is_non_negative());

                // Let's see how much energy we can obtain taking the minimum SoC and power balance into account.
                let internal_energy_differential =
                    // Usable residual energy:
                    (min_residual_energy - current_residual_energy)
                    .clamp(
                        // Limited by actual discharging power corrected by the efficiency:
                    self.battery.discharging_power.max(power_balance) / self.battery.discharging_efficiency * ONE_HOUR,
                        // The self-discharging could already drop the residual energy below the reserve:
                        KilowattHours::ZERO,
                    );
                assert!(internal_energy_differential.is_non_positive());

                // But, we actually get less from it due to the efficiency losses:
                let billable_energy_differential =
                    internal_energy_differential * self.battery.discharging_efficiency;
                assert!(billable_energy_differential.is_non_positive());

                // Calculate the payback (`max` is because the differential is negative):
                let stand_by_differential =
                    billable_energy_differential.max(self.consumption.stand_by_power * ONE_HOUR);
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
                    - grid_differential * (*rate - self.consumption.purchase_fees);
                net_profit += hour_net_profit;

                // Update current residual energy:
                current_residual_energy += internal_energy_differential;

                // Post-apply self-discharging:
                current_residual_energy -=
                    current_residual_energy * self.battery.self_discharging_rate * 0.5;
                assert!(current_residual_energy.is_non_negative());

                forecast.push(Forecast {
                    residual_energy_after: current_residual_energy,
                    grid_energy_used: grid_differential,
                    residual_energy_before,
                    net_profit: hour_net_profit,
                });
            }
        }

        let minimal_residual_energy_value = {
            let usable_residual_energy =
                forecast.last().unwrap().residual_energy_after - min_residual_energy;
            let minimal_rate = *self.hourly_rates.iter().min().unwrap();
            let minimal_selling_rate = minimal_rate - self.consumption.purchase_fees;
            if usable_residual_energy.is_non_negative() {
                // Theoretical money we can make from selling it all at once:
                usable_residual_energy * self.battery.discharging_efficiency * minimal_selling_rate
            } else {
                // Uh-oh, we need to spend money at least this much money to compensate the self-discharge:
                usable_residual_energy / self.battery.charging_efficiency * minimal_rate
            }
        };

        Outcome { net_profit, minimal_residual_energy_value, forecast }
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
            KilowattHourRate(dec!(1.0)),
            KilowattHourRate(dec!(2.0)),
            KilowattHourRate(dec!(3.0)),
            KilowattHourRate(dec!(4.0)),
            KilowattHourRate(dec!(5.0)),
        ];
        let working_mode_sequence = [
            WorkingMode::Charging,    // +3 kWh, -3 euro
            WorkingMode::Charging,    // battery is capped at 4 kWh
            WorkingMode::Balancing,   // -1 kWh, +3 euro
            WorkingMode::Discharging, //-2 kWh, +8 euro
            WorkingMode::Discharging, // battery is capped at 1 kWh
        ];
        let solar_energy = [Kilowatts(0.0); 5];
        let outcome = Simulator::builder()
            .hourly_rates(&rates)
            .solar_power(&solar_energy)
            .working_mode_sequence(&working_mode_sequence)
            .residual_energy(KilowattHours(1.0))
            .capacity(KilowattHours(4.0))
            .battery(&BatteryArgs {
                charging_power: Kilowatts(3.0),
                discharging_power: Kilowatts(-2.0),
                charging_efficiency: 1.0,
                discharging_efficiency: 1.0,
                self_discharging_rate: 0.0,
                min_soc_percent: 25,
            })
            .consumption(&ConsumptionArgs {
                stand_by_power: -Kilowatts(1.0),
                purchase_fees: KilowattHourRate(dec!(0.0)),
            })
            .build()
            .run();
        assert_eq!(outcome.net_profit.0, 8.0);
    }
}
