#![allow(clippy::doc_markdown)]
#![doc = include_str!("../../README.md")]

mod api;
mod cli;
mod core;
mod db;
mod fmt;
mod ops;
mod prelude;
mod quantity;
mod statistics;
mod tables;

use clap::{Parser, crate_version};
use sentry::integrations::{anyhow::capture_anyhow, tracing::EventFilter};
use tracing::metadata::LevelFilter;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    cli::{Args, Command},
    prelude::*,
};

fn main() -> Result {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().without_time().compact().with_filter(
            EnvFilter::builder().with_default_directive(LevelFilter::INFO.into()).from_env()?,
        ))
        .with(sentry::integrations::tracing::layer().event_filter(
            |metadata| match *metadata.level() {
                Level::ERROR => EventFilter::Event,
                _ => EventFilter::Breadcrumb,
            },
        ))
        .init();

    info!(version = crate_version!(), "starting…");
    let _ = dotenvy::dotenv();
    let args = Args::parse();
    let _sentry_guard = args.sentry.init();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async_main(args))
        .inspect_err(|error| {
            capture_anyhow(error);
        })
}

async fn async_main(args: Args) -> Result {
    match args.command {
        Command::Hunt(args) => args.run().await,
        Command::Log(args) => args.run().await,
        Command::Burrow(args) => args.run().await,
    }
}
