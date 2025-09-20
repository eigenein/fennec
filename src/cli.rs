use clap::{Parser, Subcommand};
use reqwest::Url;

use crate::units::{power::Kilowatts, rate::KilowattHourRate, surface_area::SquareMetres};

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

#[derive(Parser)]
pub struct HuntArgs {
    /// Do not push the final schedule to FoxESS Cloud (dry run).
    #[expect(clippy::doc_markdown)]
    #[clap(long)]
    pub scout: bool,

    #[clap(long = "optimization-steps", env = "OPTIMIZATION_STEPS", default_value = "1000000")]
    pub n_optimization_steps: usize,

    #[clap(long = "heartbeat-url", env = "HEARTBEAT_URL")]
    pub heartbeat_url: Option<Url>,

    #[clap(flatten)]
    pub battery: BatteryArgs,

    #[clap(flatten)]
    pub solar: SolarArgs,

    #[clap(flatten)]
    pub consumption: ConsumptionArgs,

    #[clap(flatten)]
    pub home_assistant: HomeAssistantArgs,
}

#[derive(Parser)]
pub struct ConsumptionArgs {
    /// Average stand-by household usage in kilowatts.
    #[clap(
        long = "stand-by-consumption-kilowatts",
        default_value = "0.5",
        env = "STAND_BY_CONSUMPTION_KILOWATTS"
    )]
    pub stand_by: Kilowatts,

    /// Energy purchase fees («inkoopvergoeding»).
    #[clap(long = "purchase-fees-per-kwh", default_value = "0.021", env = "PURCHASE_FEES_PER_KWH")]
    pub purchase_fees: KilowattHourRate,
}

#[derive(Parser)]
pub struct SolarArgs {
    #[clap(long = "latitude", default_value = "52.349605", env = "LATITUDE")]
    pub latitude: f64,

    #[clap(long = "longitude", default_value = "4.677388", env = "LONGITUDE")]
    pub longitude: f64,

    #[clap(long = "pv-surface-m2", default_value = "2", env = "PV_SURFACE_M2")]
    pub pv_surface: SquareMetres,

    #[clap(long = "weerlive-api-key", env = "WEERLIVE_API_KEY")]
    pub weerlive_api_key: String,
}

#[derive(Parser)]
pub struct HomeAssistantArgs {
    #[clap(
        long = "home-assistant-access-token",
        env = "HOME_ASSISTANT_ACCESS_TOKEN",
        requires = "total_energy_usage_url"
    )]
    pub access_token: Option<String>,

    #[clap(
        long = "home-assistant-total-energy-usage-url",
        env = "HOME_ASSISTANT_TOTAL_ENERGY_USAGE_URL",
        requires = "access_token"
    )]
    pub total_energy_usage_url: Option<Url>,
}

impl HomeAssistantArgs {
    pub fn into_tuple(self) -> Option<(String, Url)> {
        self.access_token.zip(self.total_energy_usage_url)
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
    /// Get parsed device variables.
    DeviceVariables,

    /// Get all device variables in raw format.
    RawDeviceVariables,

    /// Get device details.
    DeviceDetails,

    /// Get the schedule.
    Schedule,
}
