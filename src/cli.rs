use clap::{Parser, Subcommand};
use rust_decimal::Decimal;

use crate::units::{power::Kilowatts, rate::EuroPerKilowattHour};

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
    Hunt(HuntArgs),

    /// Test FoxESS Cloud API connectivity.
    #[allow(clippy::doc_markdown)]
    #[clap(name = "burrow")]
    Burrow(BurrowArgs),
}

#[derive(Parser)]
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
        default_value = "-0.8",
        env = "DISCHARGING_POWER_KILOWATTS"
    )]
    pub discharging_power: Kilowatts,

    /// Charging efficiency (look for «Battery Details» in FoxCloud app).
    #[clap(long = "charging-efficiency", default_value = "0.948", env = "CHARGING_EFFICIENCY")]
    #[allow(clippy::doc_markdown)]
    pub charging_efficiency: f64,

    /// Discharging efficiency (look for «Battery Details» in FoxCloud app).
    #[clap(
        long = "discharging-efficiency",
        default_value = "0.948",
        env = "DISCHARGING_EFFICIENCY"
    )]
    #[allow(clippy::doc_markdown)]
    pub discharging_efficiency: f64,

    /// Self-discharging rate (look for «Battery Details» in FoxCloud app).
    #[clap(long = "self-discharging-rate", default_value = "0.046", env = "SELF_DISCHARGING_RATE")]
    #[allow(clippy::doc_markdown)]
    pub self_discharging_rate: f64,

    /// Minimal state-of-charge percent.
    #[clap(long, default_value = "10", env = "MIN_SOC_PERCENT")]
    pub min_soc_percent: u32,
}

#[derive(Parser)]
pub struct HuntArgs {
    /// Do not push the final schedule to FoxESS Cloud (dry run).
    #[allow(clippy::doc_markdown)]
    #[clap(long)]
    pub scout: bool,

    #[clap(flatten)]
    pub battery: BatteryArgs,

    #[clap(flatten)]
    pub pv: PvArgs,

    #[clap(flatten)]
    pub consumption: ConsumptionArgs,
}

#[derive(Parser)]
pub struct ConsumptionArgs {
    /// Average stand-by household usage in watts, typically negative.
    #[clap(
        long = "stand-by-power-kilowatts",
        default_value = "-0.4",
        env = "STAND_BY_POWER_KILOWATTS"
    )]
    pub stand_by_power: Kilowatts,

    /// Energy purchase fees («inkoopvergoeding»).
    #[clap(long = "purchase-fees-per-kwh", default_value = "0.021", env = "PURCHASE_FEES_PER_KWH")]
    #[allow(clippy::doc_markdown)]
    pub purchase_fees: EuroPerKilowattHour,
}

#[derive(Parser)]
pub struct PvArgs {
    #[clap(long = "latitude", default_value = "52.349605", env = "LATITUDE")]
    pub latitude: Decimal,

    #[clap(long = "longitude", default_value = "4.677388", env = "LONGITUDE")]
    pub longitude: Decimal,

    #[clap(long = "pv-surface-m2", default_value = "2", env = "PV_SURFACE_M2")]
    pub pv_surface_square_meters: f64,

    #[clap(long = "weerlive-api-key", env = "WEERLIVE_API_KEY")]
    pub weerlive_api_key: String,
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
