pub mod battery;
mod burrow;
mod connection;
mod db;
mod fox_cloud;
mod go;
mod hunt;
mod log;
mod sentry;

use clap::{Parser, Subcommand};

use crate::cli::{burrow::BurrowArgs, go::GoArgs, hunt::HuntOnceArgs, sentry::SentryArgs};

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
    /// Main entry point. Run the full-featured real-time optimization service.
    #[clap(name = "go")]
    Go(Box<GoArgs>),

    /// Immediately fetch the prices and optimize the schedule once, then exit.
    #[clap(name = "hunt-once")]
    HuntOnce(Box<HuntOnceArgs>),

    /// Development tools.
    #[clap(name = "burrow")]
    Burrow(Box<BurrowArgs>),
}
