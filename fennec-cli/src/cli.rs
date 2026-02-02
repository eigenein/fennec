mod db;
mod heartbeat;
mod hunt;
mod log;

use std::{ops::RangeInclusive, path::PathBuf, time::Duration};

use chrono::{DateTime, Local, TimeDelta, Timelike};
use clap::{Parser, Subcommand};
use enumset::EnumSet;
use http::Uri;
use reqwest::Url;

pub use self::{hunt::hunt, log::log};
use crate::{
    api::home_assistant,
    cli::{db::DbArgs, heartbeat::HeartbeatArgs},
    core::{provider::Provider, working_mode::WorkingMode},
    quantity::{power::Kilowatts, rate::KilowattHourRate},
};

#[derive(Parser)]
#[command(author, version, about, propagate_version = true)]
#[must_use]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Main command: fetch the prices, optimize the schedule, and push it to the cloud.
    #[clap(name = "hunt")]
    Hunt(Box<HuntArgs>),

    /// Log meter and battery measurements.
    #[clap(name = "log")]
    Log(Box<LogArgs>),

    /// Development tools.
    #[clap(name = "burrow")]
    Burrow(Box<BurrowArgs>),
}

#[derive(Parser)]
pub struct BatteryArgs {
    #[clap(flatten)]
    pub power_limits: BatteryPowerLimits,

    #[clap(flatten)]
    pub connection: BatteryConnectionArgs,

    #[clap(flatten)]
    pub registers: BatteryRegisters,
}

#[derive(Copy, Clone, Parser)]
pub struct BatteryPowerLimits {
    /// Charging power in kilowatts.
    #[clap(
        long = "charging-power-kilowatts",
        default_value = "1.2",
        env = "CHARGING_POWER_KILOWATTS"
    )]
    pub charging_power: Kilowatts,

    /// Discharging power in kilowatts.
    #[clap(
        long = "discharging-power-kilowatts",
        default_value = "0.8",
        env = "DISCHARGING_POWER_KILOWATTS"
    )]
    pub discharging_power: Kilowatts,
}

#[derive(Parser)]
pub struct BatteryConnectionArgs {
    #[clap(long = "battery-host", env = "BATTERY_HOST")]
    pub host: String,

    #[clap(long = "battery-port", env = "BATTERY_PORT", default_value = "502")]
    pub port: u16,

    #[clap(long = "battery-slave-id", default_value = "1", env = "BATTERY_SLAVE_ID")]
    pub slave_id: u8,
}

#[derive(Copy, Clone, Parser)]
pub struct BatteryRegisters {
    #[clap(flatten)]
    pub energy: BatteryEnergyStateRegisters,

    #[clap(flatten)]
    pub setting: BatterySettingRegisters,
}

#[derive(Copy, Clone, Parser)]
pub struct BatteryEnergyStateRegisters {
    #[clap(long, default_value = "39424", env = "SOC_REGISTER")]
    pub state_of_charge: u16,

    #[clap(long, default_value = "37624", env = "SOH_REGISTER")]
    pub state_of_health: u16,

    #[clap(long, default_value = "37635", env = "DESIGN_CAPACITY_REGISTER")]
    pub design_capacity: u16,
}

#[derive(Copy, Clone, Parser)]
pub struct BatterySettingRegisters {
    #[clap(long, default_value = "46611", env = "MIN_SOC_ON_GRID_REGISTER")]
    pub min_state_of_charge_on_grid: u16,

    #[clap(long, default_value = "46610", env = "MAX_SOC_REGISTER")]
    pub max_state_of_charge: u16,
}

#[derive(Parser)]
pub struct LogArgs {
    #[clap(long, env = "BATTERY_POLLING_INTERVAL", default_value = "5s")]
    battery_polling_interval: humantime::Duration,

    #[clap(long, env = "METER_POLLING_INTERVAL", default_value = "5min")]
    meter_polling_interval: humantime::Duration,

    #[clap(long, env = "TOTAL_ENERGY_METER_URL")]
    pub total_energy_meter_url: Url,

    #[clap(long, env = "BATTERY_ENERGY_METER_URL")]
    pub battery_energy_meter_url: Url,

    #[clap(flatten)]
    pub db: DbArgs,

    #[clap(flatten)]
    pub battery_connection: BatteryConnectionArgs,

    #[clap(flatten)]
    pub battery_registers: BatteryEnergyStateRegisters,

    #[clap(flatten)]
    pub battery_heartbeat: HeartbeatArgs,
}

impl LogArgs {
    pub fn battery_polling_interval(&self) -> Duration {
        self.battery_polling_interval.into()
    }

    pub fn meter_polling_interval(&self) -> Duration {
        self.meter_polling_interval.into()
    }
}

#[derive(Parser)]
pub struct HuntArgs {
    /// Do not push the final schedule to FoxESS Cloud (dry run).
    #[expect(clippy::doc_markdown)]
    #[clap(long)]
    pub scout: bool,

    #[clap(long = "provider", env = "PROVIDER", default_value = "next-energy")]
    pub provider: Provider,

    #[clap(
        long = "working-modes",
        env = "WORKING_MODES",
        value_delimiter = ',',
        num_args = 1..,
        default_value = "backup,balance,charge",
    )]
    pub working_modes: Vec<WorkingMode>,

    /// Battery degradation rate per kilowatt-hour of the energy flow.
    #[clap(long, env = "DEGRADATION_RATE", default_value = "0")]
    pub degradation_rate: KilowattHourRate,

    #[clap(flatten)]
    pub battery: BatteryArgs,

    #[clap(flatten)]
    pub fox_ess_api: FoxEssApiArgs,

    #[clap(flatten)]
    pub estimation: EstimationArgs,

    #[clap(flatten)]
    pub db: DbArgs,

    #[clap(flatten)]
    pub heartbeat: HeartbeatArgs,
}

impl HuntArgs {
    #[must_use]
    pub fn working_modes(&self) -> EnumSet<WorkingMode> {
        self.working_modes.iter().copied().collect()
    }
}

#[derive(Parser)]
pub struct HomeAssistantArgs {
    #[clap(flatten)]
    pub connection: HomeAssistantConnectionArgs,

    #[clap(long = "home-assistant-entity-id", env = "HOME_ASSISTANT_ENTITY_ID")]
    pub entity_id: String,

    /// TODO: use `humantime`.
    #[clap(
        long = "home-assistant-history-days",
        default_value = "14",
        env = "HOME_ASSISTANT_HISTORY_DAYS"
    )]
    pub n_history_days: i64,
}

impl HomeAssistantArgs {
    pub fn history_period(&self) -> RangeInclusive<DateTime<Local>> {
        let now = Local::now();
        let now = now.with_nanosecond(0).unwrap_or(now);
        (now - TimeDelta::days(self.n_history_days))..=now
    }
}

#[derive(Parser)]
pub struct HomeAssistantConnectionArgs {
    /// Home Assistant API access token.
    #[clap(long = "home-assistant-access-token", env = "HOME_ASSISTANT_ACCESS_TOKEN")]
    pub access_token: String,

    /// Home Assistant API base URL. For example: `http://localhost:8123/api`.
    #[clap(long = "home-assistant-api-base-url", env = "HOME_ASSISTANT_API_BASE_URL")]
    pub base_url: Uri,
}

impl HomeAssistantConnectionArgs {
    pub fn new_client(&self) -> home_assistant::Api {
        home_assistant::Api::new(&self.access_token, self.base_url.clone())
    }
}

#[derive(Parser)]
pub struct FoxEssApiArgs {
    #[clap(long = "api-key", env = "FOX_ESS_API_KEY")]
    pub api_key: String,

    #[clap(long, alias = "serial", env = "FOX_ESS_SERIAL_NUMBER")]
    pub serial_number: String,
}

#[derive(Parser)]
pub struct BurrowArgs {
    #[command(subcommand)]
    pub command: BurrowCommand,
}

#[derive(Subcommand)]
pub enum BurrowCommand {
    /// Gather consumption and battery statistics.
    Statistics(Box<BurrowStatisticsArgs>),

    /// Estimate battery efficiency parameters.
    Battery(BurrowBatteryArgs),

    /// Test FoxESS Cloud API connectivity.
    FoxEss(BurrowFoxEssArgs),
}

#[derive(Parser)]
pub struct BurrowStatisticsArgs {
    #[clap(flatten)]
    pub home_assistant: HomeAssistantArgs,

    #[clap(long, env = "STATISTICS_PATH", default_value = "statistics.toml")]
    pub statistics_path: PathBuf,

    #[clap(flatten)]
    pub heartbeat: HeartbeatArgs,

    #[clap(flatten)]
    pub db: DbArgs,
}

#[derive(Parser)]
pub struct BurrowBatteryArgs {
    #[clap(flatten)]
    pub db: DbArgs,

    #[clap(flatten)]
    pub estimation: EstimationArgs,
}

#[derive(Parser)]
pub struct EstimationArgs {
    /// Measurement window duration to select from the readings when estimating battery efficiency.
    #[clap(
        long = "battery-estimation-interval",
        env = "BATTERY_ESTIMATION_INTERVAL",
        default_value = "14d"
    )]
    duration: humantime::Duration,
}

impl EstimationArgs {
    pub fn duration(&self) -> Duration {
        self.duration.into()
    }
}

#[derive(Parser)]
pub struct BurrowFoxEssArgs {
    #[clap(flatten)]
    pub fox_ess_api: FoxEssApiArgs,

    #[command(subcommand)]
    pub command: BurrowFoxEssCommand,
}

#[derive(Subcommand)]
pub enum BurrowFoxEssCommand {
    /// Get the schedule.
    Schedule,
}
