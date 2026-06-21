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

use std::{borrow::Cow, sync::Arc, time::Duration};

use backon::{ConstantBuilder, Retryable};
use chrono::{DateTime, Days, Local};
use clap::{Parser, crate_name, crate_version};
use sentry::{
    SessionMode,
    integrations::{anyhow::capture_anyhow, tracing::EventFilter},
};
use tokio::{spawn, sync::RwLock, try_join};
use tracing::metadata::LevelFilter;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

pub use self::schedule::Schedule;
use crate::{
    api::{homewizard, mini_qube},
    cli::{Args, BatteryArgs, hunter, logger},
    energy::Flow,
    math::smoothing::HalfLife,
    prelude::*,
    quantity::{price::KilowattHourPrice, time::Hours},
};

fn main() -> Result {
    init_tracing()?;

    info!(version = crate_version!(), "starting…");
    let _ = dotenvy::dotenv();
    let args = Args::parse();
    let _sentry_guard = init_sentry(args.sentry_dsn.as_deref());

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(Box::pin(run(args)))
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

async fn run(args: Args) -> Result {
    let battery_power_limits = args.battery.power_limits;
    let connections = args.connections.connect()?;

    let logger_runner = logger::Args::builder()
        .connections(connections.clone())
        .battery_power_limits(battery_power_limits)
        .energy_balance_half_life(HalfLife(Duration::from(args.energy_balance_half_life).into()))
        .battery_efficiency_half_life_factor(args.battery_efficiency_half_life_factor)
        .n_balance_harmonics(args.n_balance_harmonics)
        .build()
        .start()
        .await?;
    let hunter_runner = hunter::Runner::builder()
        .connections(connections.clone())
        .energy_provider(args.energy_provider)
        .battery_args(args.battery)
        .n_balance_harmonics(args.n_balance_harmonics)
        .dry_run(args.dry_run)
        .build();

    let hunter_state = Arc::new(RwLock::new(hunter_runner.run_once().await?));
    let state =
        web::State { hunter: hunter_state.clone(), logger: logger_runner.energy_profile.clone() };
    try_join!(
        async { spawn(logger_runner.run_forever(args.logger_cron)).await? },
        async { spawn(hunter_runner.run_forever(args.optimizer_cron, hunter_state)).await? },
        async { spawn(web::serve(args.bind.address, args.bind.port, state)).await? },
    )?;

    Ok(())
}

struct Runner {
    grid_measurement_client: homewizard::Client,
    battery_client: mini_qube::Client,
    battery_args: BatteryArgs,
    energy_provider: energy::Provider,
    n_balance_harmonics: usize,
    dry_run: bool,
    energy_balance_half_life: HalfLife<Hours>,
    battery_efficiency_half_life_factor: f64,
    //
    // Current price and battery steering schedule.
    // pub schedule: Schedule<(Flow<KilowattHourPrice>, Step)>,

    // Current solution metrics.
    // pub metrics: solution::Metrics,
    //
    /// Current energy profile.
    pub energy_profile: energy::Profile,
}

impl Runner {
    const MINI_QUBE_BACKOFF: ConstantBuilder =
        ConstantBuilder::new().with_delay(Duration::from_secs(1));

    async fn start(args: Args) -> Result<Self> {
        let this = Self {
            grid_measurement_client: args.connections.grid_measurement_url.client()?,
            battery_client: mini_qube::Client::new(args.connections.battery_address),
            battery_args: args.battery,
            energy_provider: args.energy_provider,
            n_balance_harmonics: args.n_balance_harmonics,
            dry_run: args.dry_run,
            energy_balance_half_life: HalfLife(
                Duration::from(args.energy_balance_half_life).into(),
            ),
            battery_efficiency_half_life_factor: args.battery_efficiency_half_life_factor,
            energy_profile: energy::Profile::read_from_file(args.n_balance_harmonics).await?,
        };
        Ok(this)
    }

    /// Fetch energy prices for up to 2 days.
    #[instrument(skip_all, fields(now = ?now))]
    async fn get_prices(&self, now: DateTime<Local>) -> Result<Schedule<Flow<KilowattHourPrice>>> {
        const ONE_DAY: Days = Days::new(1);

        let today = now.date_naive();
        let mut prices = self.energy_provider.get_prices(today).await?;
        ensure!(prices.len() != 0, "received empty price schedule");

        let tomorrow = today.checked_add_days(ONE_DAY).unwrap();
        prices.extend(self.energy_provider.get_prices(tomorrow).await?)?;

        info!(len = prices.len(), "fetched energy prices");
        prices.advance_to(now);
        Ok(prices)
    }

    async fn write_schedule(
        &self,
        schedule: &fennec_modbus::contrib::mini_qube::schedule::Full,
    ) -> Result {
        if self.dry_run {
            warn!("not writing the schedule to the battery, just scouting");
            for entry in schedule {
                info!(?entry.start_time, ?entry.end_time, ?entry.working_mode);
            }
        } else {
            (async || self.battery_client.write_schedule(schedule).await)
                .retry(Self::MINI_QUBE_BACKOFF)
                .notify(log_retried_error)
                .await
                .context("failed to push the schedule to the battery")?;
        }
        Ok(())
    }
}
