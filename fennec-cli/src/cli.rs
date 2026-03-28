pub mod battery;
mod burrow;
mod connection;
mod db;
mod fox_cloud;
mod hunt;
mod log;
mod sentry;

use clap::{Parser, Subcommand};

use crate::cli::{burrow::BurrowArgs, hunt::HuntOnceArgs, log::LogArgs, sentry::SentryArgs};

#[derive(Parser)]
#[command(author, version, about, propagate_version = true)]
#[must_use]
pub struct Args {
    #[clap(flatten)]
    pub sentry: SentryArgs,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Immediately fetch the prices and optimize the schedule once, then exit.
    #[clap(name = "hunt-once")]
    HuntOnce(Box<HuntOnceArgs>),

    /// Log meter and battery measurements.
    #[clap(name = "log")]
    Log(Box<LogArgs>),

    /// Development tools.
    #[clap(name = "burrow")]
    Burrow(Box<BurrowArgs>),
}
