use clap::{Parser, Subcommand};
use rust_decimal::Decimal;

use crate::units::{EuroPerKilowattHour, Kilowatts};

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
    #[clap(flatten)]
    pub power: BatteryParameters,

    /// Average stand-by household usage in watts.
    #[clap(long = "stand-by-power-watts", default_value = "400", env = "STAND_BY_POWER_WATTS")]
    pub stand_by_power_watts: u32, // FIXME: use `Watts`.

    /// Minimal state-of-charge percent.
    #[clap(long, default_value = "10", env = "MIN_SOC_PERCENT")]
    pub min_soc_percent: u32,
}

#[derive(Copy, Clone, Parser)]
pub struct BatteryParameters {
    /// Maximum charging power in kilowatts.
    #[clap(
        long = "charging-power-kilowatts",
        default_value = "1.2",
        env = "CHARGING_POWER_KILOWATTS"
    )]
    pub charging_power: Kilowatts,

    /// Maximum discharging power in kilowatts.
    #[clap(
        long = "discharging-power-kilowatts",
        default_value = "0.8",
        env = "DISCHARGING_POWER_KILOWATTS"
    )]
    pub discharging_power: Kilowatts,

    /// Round-trip efficiency (look for «Battery Details» in FoxCloud app).
    #[clap(long = "round-trip-efficiency", default_value = "0.948", env = "ROUND_TRIP_EFFICIENCY")]
    #[allow(clippy::doc_markdown)]
    pub round_trip_efficiency: Decimal,

    /// Round-trip efficiency (look for «Battery Details» in FoxCloud app).
    #[clap(long = "self-discharging-rate", default_value = "0.046", env = "SELF_DISCHARGING_RATE")]
    #[allow(clippy::doc_markdown)]
    pub self_discharging_rate: Decimal,
}

#[derive(Parser)]
pub struct HuntArgs {
    /// Do not push the final schedule to FoxESS Cloud (dry run).
    #[allow(clippy::doc_markdown)]
    #[clap(long)]
    pub scout: bool,

    #[clap(flatten)]
    pub battery: BatteryArgs,

    /// Energy purchase fees («inkoopvergoeding»).
    #[clap(long = "purchase-fees", default_value = "0.021", env = "PURCHASE_FEES")]
    #[allow(clippy::doc_markdown)]
    pub purchase_fees: EuroPerKilowattHour,
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
