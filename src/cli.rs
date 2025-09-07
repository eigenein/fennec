use clap::{Parser, Subcommand};

use crate::units::{KilowattHour, Kilowatts};

#[derive(Parser)]
#[command(author, version, about, long_about, propagate_version = true)]
pub struct Args {
    /// Pydantic Logfire token: <https://logfire.pydantic.dev/docs/how-to-guides/create-write-tokens/>.
    #[clap(long, env = "LOGFIRE_TOKEN", hide_env_values = true)]
    _logfire_token: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Main command: fetch the prices, optimize the schedule, and push it to the cloud.
    #[clap(name = "hunt")]
    Hunt(HuntArgs),

    /// Fetch current energy prices and run optimization based on manually provided battery parameters.
    #[clap(name = "scout")]
    Scout(ScoutArgs),

    /// Test FoxESS Cloud API connectivity.
    #[allow(clippy::doc_markdown)]
    #[clap(name = "burrow")]
    DebugFoxEss(DebugFoxEssArgs),
}

#[derive(Parser)]
pub struct BatteryArgs {
    #[clap(flatten)]
    pub power: BatteryPower,

    /// Average stand-by household usage in watts.
    #[clap(long = "stand-by-power-watts", default_value = "400", env = "STAND_BY_POWER_WATTS")]
    pub stand_by_power_watts: u32, // FIXME: use `Watts`.

    /// Minimal state-of-charge percent.
    #[clap(long, default_value = "10", env = "MIN_SOC_PERCENT")]
    pub min_soc_percent: u32,
}

#[derive(Copy, Clone, Parser)]
pub struct BatteryPower {
    /// Maximum charging power in kilowatts.
    #[clap(
        long = "charging-power-kilowatts",
        default_value = "1.2",
        env = "CHARGING_POWER_KILOWATTS"
    )]
    pub charging: Kilowatts,

    /// Maximum discharging power in kilowatts.
    #[clap(
        long = "discharging-power-kilowatts",
        default_value = "0.8",
        env = "DISCHARGING_POWER_KILOWATTS"
    )]
    pub discharging: Kilowatts,
}

impl BatteryPower {
    pub fn max(&self) -> Kilowatts {
        Kilowatts(self.charging.0.max(self.discharging.0))
    }
}

#[derive(Parser)]
pub struct HuntArgs {
    /// Do not push the final schedule to FoxESS Cloud (dry run).
    #[allow(clippy::doc_markdown)]
    #[clap(long, env = "STALK")]
    pub stalk: bool,

    #[clap(flatten)]
    pub fox_ess_api: FoxEssApiArgs,

    #[clap(flatten)]
    pub battery: BatteryArgs,
}

#[derive(Parser)]
pub struct ScoutArgs {
    #[clap(flatten)]
    pub battery: BatteryArgs,

    pub residual_energy: KilowattHour,

    #[clap(long, default_value = "8.4")]
    pub capacity: KilowattHour,
}

#[derive(Parser)]
pub struct FoxEssApiArgs {
    #[clap(long = "api-key", env = "FOX_ESS_API_KEY")]
    pub api_key: String,

    #[clap(long, alias = "serial", env = "FOX_ESS_SERIAL_NUMBER")]
    pub serial_number: String,
}

#[derive(Parser)]
pub struct DebugFoxEssArgs {
    #[clap(flatten)]
    pub fox_ess_api: FoxEssApiArgs,

    #[command(subcommand)]
    pub command: FoxEssCommand,
}

#[derive(Subcommand)]
pub enum FoxEssCommand {
    /// Get parsed device variables.
    DeviceVariables,

    /// Get all device variables in raw format.
    RawDeviceVariables,

    /// Get device details.
    DeviceDetails,

    /// Get the schedule.
    Schedule,
}
