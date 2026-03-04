//! Battery-related CLI arguments.

use clap::Parser;

use crate::{
    api::modbus::foxess::MQ2200,
    prelude::*,
    quantity::{power::Watts, price::KilowattHourPrice},
};

#[must_use]
#[derive(Parser)]
pub struct BatteryConnectionArgs {
    /// Battery Modbus address. Currently, only FoxESS MQ2200 is supported.
    #[clap(long = "battery-address", env = "BATTERY_ADDRESS")]
    address: String,
}

impl BatteryConnectionArgs {
    pub async fn connect(&self) -> Result<MQ2200> {
        MQ2200::connect(&self.address).await
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

    /// Battery health costs lost to the cycling, in ¤/kWh.
    #[clap(
        long = "battery-degradation-cost",
        env = "BATTERY_DEGRADATION_COST",
        default_value = "0"
    )]
    pub degradation_cost: KilowattHourPrice,
}
