use std::collections::BTreeSet;

use bon::Builder;
use rust_decimal::{Decimal, dec};

use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    prelude::*,
    strategy::{Strategy, WorkingMode},
    units::{Cost, Hours, KilowattHourRate, KilowattHours, Kilowatts},
};

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
        unique_rates.insert(minimal_buying_rate - KilowattHourRate::new(dec!(0.01)));
        unique_rates
            .insert(*unique_rates.iter().next_back().unwrap() + KilowattHourRate::new(dec!(0.01)));

        Strategy::iter_from_rates(&unique_rates)
            .map(|strategy| {
                let plan = self.simulate(strategy);
                trace!(
                    "Simulated",
                    max_charging_rate = strategy.max_charging_rate.to_string(),
                    min_discharging_rate = strategy.min_discharging_rate.to_string(),
                    profit = plan.net_profit.to_string(),
                );
                Solution { strategy, plan }
            })
            .max_by_key(|solution| solution.plan.total_profit())
            .context("there is no solution")
    }

    fn simulate(&self, strategy: Strategy) -> Plan {
        let min_residual_energy = self.capacity * f64::from(self.battery.min_soc_percent) / 100.0;

        let mut current_residual_energy = self.residual_energy;
        let mut net_profit = Cost::ZERO;
        let mut forecast = Vec::with_capacity(self.hourly_rates.len());

        for (rate, solar_power) in
            self.hourly_rates.iter().copied().zip(self.solar_power.iter().copied())
        {
            let residual_energy_before = current_residual_energy;

            // Here's what's happening at the battery connection point:
            let working_mode = if rate <= strategy.max_charging_rate {
                WorkingMode::Charging
            } else if rate >= strategy.min_discharging_rate {
                WorkingMode::Discharging
            } else {
                WorkingMode::Maintain
            };
            let power_balance = match working_mode {
                WorkingMode::Charging => self.battery.charging_power,
                WorkingMode::Discharging => self.battery.discharging_power,
                WorkingMode::Balancing => solar_power + self.consumption.stand_by_power,
                WorkingMode::Maintain => Kilowatts::ZERO,
            };

            // Charging:
            if power_balance.0.is_sign_positive() {
                // Let's see how much energy is spent charging it taking the power balance and capacity into account:
                let billable_energy_differential = (self.capacity - current_residual_energy)
                    .min(self.battery.charging_power.min(power_balance) * Hours::ONE);
                assert!(billable_energy_differential.is_non_negative());

                // Calculate the distribution between the available grid and PV energy:
                let pv_energy_used = (solar_power * Hours::ONE).min(billable_energy_differential);
                let grid_energy_used = billable_energy_differential - pv_energy_used;
                assert!(pv_energy_used.is_non_negative());
                assert!(grid_energy_used.is_non_negative());

                // Calculate the associated costs:
                let hour_net_profit =
                    // For PV energy, we estimate the lost profit without the purchase fees:
                    -pv_energy_used * (rate - self.consumption.purchase_fees)
                    // For grid energy, we are buying it at the full rate:
                    - grid_energy_used * rate;
                net_profit += hour_net_profit;

                // Update current residual energy taking the efficiency into account:
                current_residual_energy +=
                    billable_energy_differential * self.battery.charging_efficiency;

                forecast.push(HourStep {
                    residual_energy_after: current_residual_energy,
                    grid_energy_used,
                    residual_energy_before,
                    net_profit: hour_net_profit,
                    working_mode,
                });
            }
            // Discharging:
            else {
                // Let's see how much energy we can obtain taking the minimum SoC and power balance into account.
                let internal_energy_differential =
                    // Usable residual energy:
                    (min_residual_energy - current_residual_energy)
                    .clamp(
                        // Limited by actual discharging power corrected by the efficiency:
                    self.battery.discharging_power.max(power_balance) / self.battery.discharging_efficiency * Hours::ONE,
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
                    billable_energy_differential.max(self.consumption.stand_by_power * Hours::ONE);
                let grid_differential = billable_energy_differential - stand_by_differential;
                assert!(stand_by_differential.is_non_positive());
                assert!(
                    grid_differential.is_non_positive(),
                    "grid differential: {grid_differential}",
                );
                let hour_net_profit =
                    // Equivalent stand-by consumption from the grid would be billed with the full rate:
                    -stand_by_differential * rate
                    // The rest we sell a little cheaper, without the purchase fees:
                    - grid_differential * (rate - self.consumption.purchase_fees);
                net_profit += hour_net_profit;

                // Update current residual energy:
                current_residual_energy += internal_energy_differential;

                forecast.push(HourStep {
                    residual_energy_after: current_residual_energy,
                    grid_energy_used: grid_differential,
                    residual_energy_before,
                    net_profit: hour_net_profit,
                    working_mode,
                });
            }
        }

        let residual_energy_value = {
            let usable_residual_energy =
                forecast.last().unwrap().residual_energy_after - min_residual_energy;
            let average_buying_rate = self.hourly_rates.iter().copied().sum::<KilowattHourRate>()
                / Decimal::from(self.hourly_rates.len());
            let average_selling_rate = average_buying_rate - self.consumption.purchase_fees;
            if usable_residual_energy.is_non_negative() {
                // Theoretical money we can make from selling it all at once:
                usable_residual_energy * self.battery.discharging_efficiency * average_selling_rate
            } else {
                // Uh-oh, we need to spend at least this much money to compensate the self-discharge:
                usable_residual_energy / self.battery.charging_efficiency * average_buying_rate
            }
        };

        Plan { net_profit, residual_energy_value, steps: forecast }
    }
}

/// Optimization plan that describes how the battery will work in the upcoming hours.
pub struct Plan {
    /// Calculated profit.
    pub net_profit: Cost,

    /// Minimal selling cost of the residual energy by the end of the simulation.
    ///
    /// It may be negative, that would mean losses due to the self-discharge below the minimal SoC.
    #[expect(clippy::doc_markdown)]
    pub residual_energy_value: Cost,

    /// Hourly forecast.
    pub steps: Vec<HourStep>,
}

impl Plan {
    /// Sum of the net profit and the residual energy value.
    pub fn total_profit(&self) -> Cost {
        self.net_profit + self.residual_energy_value
    }
}

/// Single-hour working plan step.
pub struct HourStep {
    pub working_mode: WorkingMode,
    pub residual_energy_before: KilowattHours,
    pub residual_energy_after: KilowattHours,
    pub grid_energy_used: KilowattHours,
    pub net_profit: Cost,
}

pub struct Solution {
    pub strategy: Strategy,
    pub plan: Plan,
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;
    use crate::cli::BatteryArgs;

    #[test]
    fn test_simulate() {
        let rates = [
            KilowattHourRate::new(dec!(1.0)),
            KilowattHourRate::new(dec!(2.0)),
            KilowattHourRate::new(dec!(3.0)),
            KilowattHourRate::new(dec!(4.0)),
            KilowattHourRate::new(dec!(5.0)),
        ];
        let outcome = Optimizer::builder()
            .hourly_rates(&rates)
            .solar_power(&[Kilowatts::ZERO; 5])
            .residual_energy(KilowattHours::new(1.0))
            .capacity(KilowattHours::new(4.0))
            .battery(&BatteryArgs {
                charging_power: Kilowatts::new(3.0),
                discharging_power: Kilowatts::new(-2.0),
                charging_efficiency: 1.0,
                discharging_efficiency: 1.0,
                min_soc_percent: 25,
            })
            .consumption(&ConsumptionArgs {
                stand_by_power: -Kilowatts::new(1.0),
                purchase_fees: KilowattHourRate::new(dec!(0.0)),
            })
            .build()
            .simulate(Strategy {
                max_charging_rate: KilowattHourRate::new(dec!(2.0)),
                min_discharging_rate: KilowattHourRate::new(dec!(4.0)),
            });

        assert_eq!(
            outcome.steps.iter().map(|step| step.working_mode).collect::<Vec<_>>(),
            [
                WorkingMode::Charging, // +3 kWh, -3 euro
                WorkingMode::Charging, // battery is capped at 4 kWh
                WorkingMode::Maintain,
                WorkingMode::Discharging, // -2 kWh, +8 euro
                WorkingMode::Discharging, // -1 kWh, +5 euro, battery is further capped at 1 kWh
            ]
        );
        assert_eq!(outcome.net_profit.0, 10.0);
    }
}
