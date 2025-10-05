use clap::{Parser, Subcommand};
use reqwest::Url;

use crate::{
    api::home_assistant,
    prelude::*,
    quantity::{power::Kilowatts, rate::KilowattHourRate},
};

#[derive(Parser)]
#[command(author, version, about, propagate_version = true)]
pub struct Args {
    /// Pydantic Logfire token: <https://logfire.pydantic.dev/docs/how-to-guides/create-write-tokens/>.
    #[clap(long, env = "LOGFIRE_TOKEN", hide_env_values = true)]
    _logfire_token: Option<String>,

    #[clap(flatten)]
    pub fox_ess_api: FoxEssApiArgs,

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
    Burrow(BurrowArgs),
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
}

#[derive(Parser)]
pub struct HuntArgs {
    /// Do not push the final schedule to FoxESS Cloud (dry run).
    #[expect(clippy::doc_markdown)]
    #[clap(long)]
    pub scout: bool,

    #[clap(long = "heartbeat-url", env = "HEARTBEAT_URL")]
    pub heartbeat_url: Option<Url>,

    #[clap(flatten)]
    pub battery: BatteryArgs,

    #[clap(flatten)]
    pub consumption: ConsumptionArgs,

    #[clap(flatten)]
    pub home_assistant: HomeAssistantArgs,
}

#[derive(Copy, Clone, Parser)]
pub struct ConsumptionArgs {
    /// Energy purchase fees («inkoopvergoeding»).
    #[clap(long = "purchase-fees-per-kwh", default_value = "0.021", env = "PURCHASE_FEES_PER_KWH")]
    pub purchase_fees: KilowattHourRate,
}

#[derive(Parser)]
pub struct HomeAssistantArgs {
    #[clap(flatten)]
    pub connection: HomeAssistantConnectionArgs,

    #[clap(
        long = "home-assistant-battery-state-entity-id",
        env = "HOME_ASSISTANT_BATTERY_STATE_ENTITY_ID"
    )]
    pub battery_state_entity_id: String,

    #[clap(
        long = "home-assistant-total-usage-entity-id",
        env = "HOME_ASSISTANT_TOTAL_USAGE_ENTITY_ID"
    )]
    pub total_usage_entity_id: String,

    #[clap(
        long = "home-assistant-solar-yield-entity-id",
        env = "HOME_ASSISTANT_SOLAR_YIELD_ENTITY_ID"
    )]
    pub solar_yield_entity_id: String,

    #[clap(
        long = "home-assistant-history-days",
        default_value = "14",
        env = "HOME_ASSISTANT_HISTORY_DAYS"
    )]
    pub n_history_days: i64,
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
    pub fn try_new_client(self) -> Result<home_assistant::Api> {
        home_assistant::Api::try_new(&self.access_token, self.base_url)
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

#[derive(Parser)]
pub struct BurrowFoxEssArgs {
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

#[derive(Subcommand)]
pub enum BurrowCommand {
    /// Test FoxESS Cloud API connectivity.
    FoxEss(BurrowFoxEssArgs),

    /// Fetch and dump the battery differential history from Home Assistant.
    BatteryDifferentials(BurrowBatteryDifferentialsArgs),
}

#[derive(Parser)]
pub struct BurrowBatteryDifferentialsArgs {
    #[clap(flatten)]
    pub home_assistant: HomeAssistantArgs,
}
