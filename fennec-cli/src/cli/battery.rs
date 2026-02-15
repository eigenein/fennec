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
    pub state_of_charge: modbus::ParsedUrl,

    #[clap(long, env = "BATTERY_STATE_OF_HEALTH_URL")]
    pub state_of_health: modbus::ParsedUrl,

    #[clap(long, env = "BATTERY_DESIGN_CAPACITY_URL")]
    pub design_capacity: modbus::ParsedUrl,
}

impl BatteryEnergyStateUrls {
    pub async fn read(&self) -> Result<BatteryEnergyState> {
        Ok(BatteryEnergyState {
            design_capacity: u16::try_from(self.design_capacity.read().await?)?.into(),
            state_of_charge: u16::try_from(self.state_of_charge.read().await?)?.into(),
            state_of_health: u16::try_from(self.state_of_health.read().await?)?.into(),
        })
    }
}

#[derive(Parser)]
pub struct BatterySettingUrls {
    #[clap(long, env = "BATTERY_MIN_STATE_OF_CHARGE_URL")]
    pub min_state_of_charge: modbus::ParsedUrl,

    #[clap(long, env = "BATTERY_MAX_STATE_OF_CHARGE_URL")]
    pub max_state_of_charge: modbus::ParsedUrl,
}

impl BatterySettingUrls {
    pub async fn read(&self) -> Result<BatterySettings> {
        let min_state_of_charge = u16::try_from(self.min_state_of_charge.read().await?)?.into();
        let max_state_of_charge = u16::try_from(self.max_state_of_charge.read().await?)?.into();
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
