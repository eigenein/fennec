//! Battery-related CLI arguments.

use clap::Parser;

use crate::{
    api::{
        modbus,
        modbus::{BatteryEnergyState, BatterySettings, BatteryState},
    },
    ops::RangeInclusive,
    prelude::*,
    quantity::power::Kilowatts,
};

#[derive(Parser)]
pub struct BatteryConnectionArgs {
    #[clap(flatten)]
    pub energy: BatteryEnergyStateUrls,

    #[clap(flatten)]
    pub setting: BatterySettingUrls,
}

impl BatteryConnectionArgs {
    pub async fn read(&self) -> Result<BatteryState> {
        Ok(BatteryState { energy: self.energy.read().await?, settings: self.setting.read().await? })
    }
}

#[derive(Parser)]
pub struct BatteryEnergyStateUrls {
    #[clap(long, env = "BATTERY_STATE_OF_CHARGE_URL")]
    pub state_of_charge: modbus::Url,

    #[clap(long, env = "BATTERY_STATE_OF_HEALTH_URL")]
    pub state_of_health: modbus::Url,

    #[clap(long, env = "BATTERY_DESIGN_CAPACITY_URL")]
    pub design_capacity: modbus::Url,
}

impl BatteryEnergyStateUrls {
    pub async fn read(&self) -> Result<BatteryEnergyState> {
        Ok(BatteryEnergyState {
            design_capacity: modbus::connect(&self.design_capacity).await?.read::<u16, _>().await?,
            state_of_charge: modbus::connect(&self.state_of_charge).await?.read::<u16, _>().await?,
            state_of_health: modbus::connect(&self.state_of_health).await?.read::<u16, _>().await?,
        })
    }
}

#[derive(Parser)]
pub struct BatterySettingUrls {
    #[clap(long, env = "BATTERY_MIN_STATE_OF_CHARGE_URL")]
    pub min_state_of_charge: modbus::Url,

    #[clap(long, env = "BATTERY_MAX_STATE_OF_CHARGE_URL")]
    pub max_state_of_charge: modbus::Url,
}

impl BatterySettingUrls {
    pub async fn read(&self) -> Result<BatterySettings> {
        let min_state_of_charge =
            modbus::connect(&self.min_state_of_charge).await?.read::<u16, _>().await?;
        let max_state_of_charge =
            modbus::connect(&self.max_state_of_charge).await?.read::<u16, _>().await?;
        Ok(BatterySettings {
            allowed_state_of_charge: RangeInclusive::from_std(
                min_state_of_charge..=max_state_of_charge,
            ),
        })
    }
}

#[derive(Copy, Clone, Parser)]
pub struct BatteryPowerLimits {
    /// Charging power in kilowatts.
    #[clap(
        long = "charging-power-kilowatts",
        default_value = "1.2",
        env = "CHARGING_POWER_KILOWATTS"
    )]
    pub charging: Kilowatts,

    /// Discharging power in kilowatts.
    #[clap(
        long = "discharging-power-kilowatts",
        default_value = "0.8",
        env = "DISCHARGING_POWER_KILOWATTS"
    )]
    pub discharging: Kilowatts,
}

#[derive(Parser)]
pub struct BatteryArgs {
    #[clap(flatten)]
    pub power_limits: BatteryPowerLimits,

    #[clap(flatten)]
    pub connection: BatteryConnectionArgs,
}
