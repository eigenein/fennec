#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_panics_doc)]
#![doc = include_str!("../../README.md")]

mod api;
mod battery;
mod cli;
mod cron;
mod energy;
mod math;
mod ops;
mod prelude;
mod quantity;
mod schedule;
mod solution;
mod web;

use std::borrow::Cow;

use clap::{Parser, crate_name, crate_version};
use sentry::{
    SessionMode,
    integrations::{anyhow::capture_anyhow, tracing::EventFilter},
};
use tracing::metadata::LevelFilter;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

pub use self::schedule::Schedule;
use crate::{cli::Args, prelude::*};

fn main() -> Result {
    init_tracing()?;

    info!(version = crate_version!(), "starting…");
    let _ = dotenvy::dotenv();
    let args = Args::parse();
    let _sentry_guard = init_sentry(args.sentry_dsn.as_deref());

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(Box::pin(args.run()))
        .inspect_err(|error| {
            capture_anyhow(error);
        })
}

fn init_tracing() -> Result {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()?
        .add_directive("h2=warn".parse()?);
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().without_time().compact().with_filter(env_filter))
        .with(sentry::integrations::tracing::layer().event_filter(
            |metadata| match *metadata.level() {
                Level::ERROR => EventFilter::Event,
                _ => EventFilter::Breadcrumb,
            },
        ))
        .init();
    Ok(())
}

fn init_sentry(dsn: Option<&str>) -> sentry::ClientInitGuard {
    let options = sentry::ClientOptions {
        traces_sample_rate: 1.0,
        sample_rate: 1.0,
        send_default_pii: true,
        attach_stacktrace: true,
        in_app_include: vec![crate_name!()],
        release: Some(Cow::Borrowed(crate_version!())),
        auto_session_tracking: true,
        session_mode: SessionMode::Application,
        ..Default::default()
    };
    let guard = sentry::init((dsn, options));
    if !guard.is_enabled() {
        warn!("Sentry is disabled");
    }
    guard
}
