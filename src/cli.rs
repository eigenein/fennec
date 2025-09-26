use clap::{Parser, Subcommand};
use reqwest::Url;

use crate::quantity::{power::Kilowatts, rate::KilowattHourRate};

#[derive(Parser)]
#[command(author, version, about, long_about, propagate_version = true)]
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

    /// Test FoxESS Cloud API connectivity.
    #[expect(clippy::doc_markdown)]
    #[clap(name = "burrow")]
    Burrow(BurrowArgs),
}

#[derive(Copy, Clone, Parser)]
pub struct BatteryArgs {
    /// Maximum external charging power in kilowatts.
    #[clap(
        long = "charging-power-kilowatts",
        default_value = "1.2",
        env = "CHARGING_POWER_KILOWATTS"
    )]
    pub charging_power: Kilowatts,

    /// Maximum external discharging power in kilowatts, negative.
    #[clap(
        long = "discharging-power-kilowatts",
        default_value = "0.8",
        env = "DISCHARGING_POWER_KILOWATTS"
    )]
    pub discharging_power: Kilowatts,

    #[clap(long = "battery-efficiency", default_value = "0.94", env = "BATTERY_EFFICIENCY")]
    pub efficiency: f64,

    /// Minimal state-of-charge percent.
    #[clap(long, default_value = "10", env = "MIN_SOC_PERCENT")]
    pub min_soc_percent: u32,

    #[clap(long = "battery-self-discharge", default_value = "0.02", env = "SELF_DISCHARGE")]
    pub self_discharge: Kilowatts,
}

impl BatteryArgs {
    pub fn max_feed_power_watts(&self) -> u32 {
        // FIXME: maybe, it's better to have the options as `u32` and convert them later.
        self.charging_power.max(self.discharging_power).into_watts_u32()
    }
}

#[derive(Parser)]
pub struct HuntArgs {
    #[clap(long = "mongodb-url", env = "MONGODB_URL")]
    pub mongodb_url: Url,

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
    #[clap(long = "home-assistant-access-token", env = "HOME_ASSISTANT_ACCESS_TOKEN")]
    pub access_token: String,

    #[clap(
        long = "home-assistant-total-energy-usage-url",
        env = "HOME_ASSISTANT_TOTAL_ENERGY_USAGE_URL"
    )]
    pub total_energy_usage_url: Url,
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
    /// Get parsed device variables.
    DeviceVariables,

    /// Get all device variables in raw format.
    RawDeviceVariables,

    /// Get device details.
    DeviceDetails,

    /// Get the schedule.
    Schedule,
}
