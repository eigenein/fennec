#![allow(clippy::doc_markdown)]
#![doc = include_str!("../../README.md")]

mod api;
mod cli;
mod core;
mod db;
mod fmt;
mod prelude;
mod quantity;
mod statistics;
mod tables;

use clap::{Parser, crate_version};

use crate::{
    cli::{Args, Command},
    prelude::*,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt().without_time().compact().init();
    info!(version = crate_version!(), "starting…");

    match Args::parse().command {
        Command::Hunt(args) => args.hunt().await,
        Command::Log(args) => args.log().await,
        Command::Burrow(args) => args.burrow().await,
    }
}
