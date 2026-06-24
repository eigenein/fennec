use clap::Parser;

use crate::{energy, quantity::power::Watts};

/// Battery power limits.
///
/// TODO: we could use `Watts<u16>` here.
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

    /// Inverter power limit in watts – limits the summed grid and EPS output when discharging.
    #[clap(
        name = "max_inverter_power_watts",
        long = "max-inverter-power-watts",
        default_value = "1200",
        env = "MAX_INVERTER_POWER_WATTS"
    )]
    pub max_inverter_power: Watts,
}

impl PowerLimits {
    /// Calculate the effective power limits given the average EPS power.
    pub fn max_effective_flow(self, average_eps_power: Watts) -> energy::Flow<Watts> {
        energy::Flow {
            import: self.charging,

            // EPS power does not compete with the grid output, hence adding it on top.
            // The total discharging power, however, is limited by the maximum inverter output.
            export: (self.discharging + average_eps_power).min(self.max_inverter_power),
        }
    }
}
