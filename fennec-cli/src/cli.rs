pub mod battery;
mod connection;
mod hunt;
mod log;
mod run;
mod sentry;
pub mod state;
mod web;

use clap::{Parser, Subcommand};

use crate::cli::{run::RunArgs, sentry::SentryArgs};

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
}
