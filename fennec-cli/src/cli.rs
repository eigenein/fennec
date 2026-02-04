pub mod battery;
mod burrow;
mod db;
mod estimation;
mod foxess;
mod hunt;
mod log;

use clap::{Parser, Subcommand};

pub use self::estimation::WeightMode;
use crate::cli::{burrow::BurrowArgs, hunt::HuntArgs, log::LogArgs};

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
