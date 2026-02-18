//! Battery-related CLI arguments.

use clap::Parser;

use crate::{
    api::{modbus, modbus::foxess},
    prelude::*,
    quantity::power::Watts,
};

#[must_use]
#[derive(Parser)]
pub struct BatteryConnectionArgs {
    #[clap(flatten)]
    pub energy: BatteryEnergyStateUrls,

    /// Modbus URL for the battery minimum SoC percentage setting.
    #[clap(long = "battery-min-state-of-charge-url", env = "BATTERY_MIN_STATE_OF_CHARGE_URL")]
    pub min_state_of_charge: modbus::ParsedUrl,

    /// Modbus URL for the battery maximum SoC percentage setting.
    #[clap(long = "battery-max-state-of-charge-url", env = "BATTERY_MAX_STATE_OF_CHARGE_URL")]
    pub max_state_of_charge: modbus::ParsedUrl,
}

impl BatteryConnectionArgs {
    pub async fn connect(&self) -> Result<foxess::Clients> {
        Ok(foxess::Clients {
            energy_state: self.energy.connect().await?,
            min_state_of_charge: self.min_state_of_charge.connect().await?,
            max_state_of_charge: self.max_state_of_charge.connect().await?,
        })
    }
}

#[must_use]
#[derive(Parser)]
pub struct BatteryEnergyStateUrls {
    /// Modbus URL for the battery state of charge percentage.
    #[clap(long = "battery-state-of-charge-url", env = "BATTERY_STATE_OF_CHARGE_URL")]
    pub state_of_charge: modbus::ParsedUrl,

    /// Modbus URL for the battery state of health percentage.
    #[clap(long = "battery-min-state-of-health-url", env = "BATTERY_STATE_OF_HEALTH_URL")]
    pub state_of_health: modbus::ParsedUrl,

    /// Modbus URL for the battery design capacity in decawatt-hours.
    #[clap(long = "battery-design-capacity-url", env = "BATTERY_DESIGN_CAPACITY_URL")]
    pub design_capacity: modbus::ParsedUrl,
}

impl BatteryEnergyStateUrls {
    pub async fn connect(&self) -> Result<foxess::EnergyStateClients> {
        Ok(foxess::EnergyStateClients {
            state_of_charge: self.state_of_charge.connect().await?,
            state_of_health: self.state_of_health.connect().await?,
            design_capacity: self.design_capacity.connect().await?,
        })
    }
}

#[must_use]
#[derive(Copy, Clone, Parser)]
pub struct BatteryPowerLimits {
    /// Charging power in watts.
    #[clap(long = "charging-power-watts", default_value = "1200", env = "CHARGING_POWER_WATTS")]
    pub charging: Watts,

    /// Discharging power in watts.
    #[clap(
        long = "discharging-power-watts",
        default_value = "800",
        env = "DISCHARGING_POWER_WATTS"
    )]
    pub discharging: Watts,
}

#[derive(Parser)]
pub struct BatteryArgs {
    #[clap(flatten)]
    pub power_limits: BatteryPowerLimits,

    #[clap(flatten)]
    pub connection: BatteryConnectionArgs,
}
