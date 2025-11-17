use std::{ops::RangeInclusive, path::PathBuf};

use chrono::{DateTime, Local, TimeDelta, Timelike};
use clap::{Parser, Subcommand};
use enumset::EnumSet;
use reqwest::Url;

use crate::{
    api::home_assistant,
    core::working_mode::WorkingMode,
    prelude::*,
    quantity::{power::Kilowatts, rate::KilowattHourRate},
};

#[derive(Parser)]
#[command(author, version, about, propagate_version = true)]
pub struct Args {
    #[clap(long = "heartbeat-url", env = "HEARTBEAT_URL")]
    pub heartbeat_url: Option<Url>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Main command: fetch the prices, optimize the schedule, and push it to the cloud.
    #[clap(name = "hunt")]
    Hunt(Box<HuntArgs>),

    /// Development tools.
    #[clap(name = "burrow")]
    Burrow(Box<BurrowArgs>),
}

#[derive(Copy, Clone, Parser)]
pub struct BatteryArgs {
    /// Charging power in kilowatts.
    ///
    /// TODO: split into «technical» and «actual» (1185 W).
    #[clap(
        long = "charging-power-kilowatts",
        default_value = "1.2",
        env = "CHARGING_POWER_KILOWATTS"
    )]
    pub charging_power: Kilowatts,

    /// Discharging power in kilowatts.
    ///
    /// TODO: split into «technical» and «actual» (825 W).
    #[clap(
        long = "discharging-power-kilowatts",
        default_value = "0.8",
        env = "DISCHARGING_POWER_KILOWATTS"
    )]
    pub discharging_power: Kilowatts,

    /// Minimal state-of-charge percent.
    #[clap(long, default_value = "10", env = "MIN_SOC_PERCENT")]
    pub min_soc_percent: u32,

    #[clap(flatten)]
    pub parameters: BatteryParameters,
}

#[derive(Copy, Clone, Parser)]
pub struct BatteryParameters {
    #[clap(long = "battery-parasitic-load", env = "BATTERY_PARASITIC_LOAD")]
    pub parasitic_load: Kilowatts,

    #[clap(long = "battery-charging-efficiency", env = "BATTERY_CHARGING_EFFICIENCY")]
    pub charging_efficiency: f64,

    #[clap(long = "battery-discharging-efficiency", env = "BATTERY_DISCHARGING_EFFICIENCY")]
    pub discharging_efficiency: f64,
}

#[derive(Parser)]
pub struct HuntArgs {
    /// Do not push the final schedule to FoxESS Cloud (dry run).
    #[expect(clippy::doc_markdown)]
    #[clap(long)]
    pub scout: bool,

    #[clap(
        long = "working-modes",
        env = "WORKING_MODES",
        value_delimiter = ',',
        num_args = 1..,
        default_value = "backup,balance,charge-slowly,charge",
    )]
    pub working_modes: Vec<WorkingMode>,

    /// Energy purchase fees («inkoopvergoeding»).
    #[clap(long = "purchase-fee-per-kwh", default_value = "0.021", env = "PURCHASE_FEE_PER_KWH")]
    pub purchase_fee: KilowattHourRate,

    /// TODO: remove in favour of `burrow stats`.
    #[clap(flatten)]
    pub battery: BatteryArgs,

    /// TODO: remove in favour of `burrow stats`.
    #[clap(flatten)]
    pub home_assistant: HomeAssistantArgs,

    #[clap(flatten)]
    pub fox_ess_api: FoxEssApiArgs,
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
    pub base_url: Url,
}

impl HomeAssistantConnectionArgs {
    pub fn try_new_client(&self) -> Result<home_assistant::Api<'_>> {
        home_assistant::Api::try_new(&self.access_token, &self.base_url)
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
    Statistics(BurrowStatisticsArgs),

    /// Test FoxESS Cloud API connectivity.
    FoxEss(BurrowFoxEssArgs),
}

#[derive(Parser)]
pub struct BurrowStatisticsArgs {
    #[clap(flatten)]
    pub home_assistant: HomeAssistantArgs,

    #[clap(long, env = "STATISTICS_PATH", default_value = "statistics.toml")]
    pub output_file: PathBuf,
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
    /// Get parsed device variables.
    DeviceVariables,

    /// Get all device variables in raw format.
    RawDeviceVariables,

    /// Get device details.
    DeviceDetails,

    /// Get the schedule.
    Schedule,
}
