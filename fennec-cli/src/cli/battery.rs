//! Battery-related CLI arguments.

use clap::Parser;

use crate::{
    energy,
    quantity::{power::Watts, price::KilowattHourPrice, ratios::Percentage},
};

#[must_use]
#[derive(Copy, Clone, Parser)]
pub struct PowerLimits {
    /// Charging power in watts.
    #[clap(
        name = "charging_power",
        long = "charging-power-watts",
        default_value = "1200",
        env = "CHARGING_POWER_WATTS"
    )]
    pub charging: Watts,

    /// Discharging power in watts.
    #[clap(
        name = "discharging_power",
        long = "discharging-power-watts",
        default_value = "800",
        env = "DISCHARGING_POWER_WATTS"
    )]
    pub discharging: Watts,

    /// Inverter output power limit in watts – limits the summed grid and EPS output when discharging.
    #[clap(
        name = "max_inverter_output_watts",
        long = "max-inverter-output-watts",
        default_value = "1200",
        env = "MAX_INVERTER_OUTPUT_WATTS"
    )]
    pub max_inverter_output: Watts,
}

impl PowerLimits {
    /// Calculate the effective power limits giving the average EPS power.
    pub fn max_effective_flow(self, average_eps_power: Watts) -> energy::Flow<Watts> {
        energy::Flow {
            import: self.charging,

            // EPS power does not compete with the grid output, hence adding it on top.
            // The total discharging power, however, is limited by the maximum inverter output.
            export: (self.discharging + average_eps_power).min(self.max_inverter_output),
        }
    }
}

#[must_use]
#[derive(Copy, Clone, Parser)]
pub struct ChargeLimits {
    /// Minimum allowed state-of-charge in percents.
    #[clap(
        long = "battery-min-state-of-charge",
        env = "BATTERY_MIN_STATE_OF_CHARGE",
        default_value = "10"
    )]
    pub min: Percentage,

    /// Maximum allowed state-of-charge in percents.
    #[clap(
        long = "battery-max-state-of-charge",
        env = "BATTERY_MAX_STATE_OF_CHARGE",
        default_value = "100"
    )]
    pub max: Percentage,
}

#[derive(Parser)]
pub struct BatteryArgs {
    #[clap(flatten)]
    pub power_limits: PowerLimits,

    #[clap(flatten)]
    pub charge_limits: ChargeLimits,

    /// Battery health costs lost to the cycling, in ¤/kWh.
    #[clap(
        long = "battery-degradation-cost",
        env = "BATTERY_DEGRADATION_COST",
        default_value = "0.01"
    )]
    pub degradation_cost: KilowattHourPrice,
}
