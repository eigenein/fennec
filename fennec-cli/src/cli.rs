pub mod battery;
mod burrow;
mod connection;
mod db;
mod hunt;
mod log;
mod run;
mod sentry;

use clap::{Parser, Subcommand};

use crate::cli::{burrow::BurrowArgs, run::RunArgs, sentry::SentryArgs};

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
    #[clap(name = "run")]
    Run(Box<RunArgs>),

    /// Development tools.
    #[clap(name = "burrow")]
    Burrow(Box<BurrowArgs>),
}
