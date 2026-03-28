//! Battery-related CLI arguments.

use clap::Parser;

use crate::{
    energy,
    quantity::{power::Watts, price::KilowattHourPrice},
};

#[must_use]
#[derive(Copy, Clone, Parser)]
pub struct BatteryPowerLimits {
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

impl BatteryPowerLimits {
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

#[derive(Parser)]
pub struct BatteryArgs {
    #[clap(flatten)]
    pub power_limits: BatteryPowerLimits,

    /// Battery health costs lost to the cycling, in ¤/kWh.
    #[clap(
        long = "battery-degradation-cost",
        env = "BATTERY_DEGRADATION_COST",
        default_value = "0"
    )]
    pub degradation_cost: KilowattHourPrice,

    #[clap(flatten)]
    pub efficiency: Efficiency,
}

#[derive(Copy, Clone, Parser)]
pub struct Efficiency {
    /// Parasitic load in Watts, as measured from register 39237 (negative) when idling.
    #[clap(long = "battery-parasitic-load-watts", default_value = "14")]
    pub parasitic_load: Watts,

    /// Charging efficiency as measured from register 39237 (positive) to 39134 (negative) when charging.
    #[clap(
        name = "charging_efficiency",
        long = "battery-charging-efficiency",
        default_value = "0.929"
    )]
    pub charging: f64,

    /// Discharging efficiency as measured from register 39134 (positive) to 39237 (negative) when discharging.
    #[clap(
        name = "discharging_efficiency",
        long = "battery-discharging-efficiency",
        default_value = "0.84"
    )]
    pub discharging: f64,
}
