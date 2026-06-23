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
use chrono::Local;
use clap::{Parser, crate_name, crate_version};
use sentry::{
    SessionMode,
    integrations::{anyhow::capture_anyhow, tracing::EventFilter},
};
use tokio::{spawn, sync::RwLock, time::MissedTickBehavior, try_join};
use tracing::metadata::LevelFilter;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

pub use self::schedule::Schedule;
use crate::{
    api::{homewizard, mini_qube},
    cli::{Args, BatteryPowerLimits, Connections, hunter},
    math::smoothing::HalfLife,
    prelude::*,
    quantity::{power::Watts, time::Hours},
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
    let logger_runner = Logger::start(&args).await?;
    let hunter_runner = hunter::Runner::builder()
        .connections(args.connections.connect()?)
        .energy_provider(args.energy_provider)
        .battery_args(args.battery)
        .n_balance_harmonics(args.n_balance_harmonics)
        .dry_run(args.dry_run)
        .build();

    let hunter_state = Arc::new(RwLock::new(hunter_runner.run_once().await?));
    let state =
        web::State { hunter: hunter_state.clone(), logger: logger_runner.energy_profile.clone() };
    try_join!(
        async { spawn(logger_runner.run_forever(args.logger_interval.into())).await? },
        async { spawn(hunter_runner.run_forever(args.optimizer_cron, hunter_state)).await? },
        async { spawn(web::serve(args.bind.address, args.bind.port, state)).await? },
    )?;

    Ok(())
}

#[must_use]
pub struct Logger {
    connections: Connections,
    battery_power_limits: BatteryPowerLimits,
    energy_balance_half_life: HalfLife<Hours>,
    battery_efficiency_half_life_factor: f64,
    pub energy_profile: Arc<RwLock<energy::Profile>>,
}

impl Logger {
    const BACKOFF: ConstantBuilder = ConstantBuilder::new().with_delay(Duration::from_secs(1));

    #[instrument(skip_all)]
    pub async fn start(args: &Args) -> Result<Self> {
        let energy_profile = energy::Profile::read_from_file(args.n_balance_harmonics).await?;
        let this = Self {
            connections: args.connections.connect()?,
            battery_power_limits: args.battery.power_limits,
            energy_balance_half_life: HalfLife(
                Duration::from(args.energy_balance_half_life).into(),
            ),
            battery_efficiency_half_life_factor: args.battery_efficiency_half_life_factor,
            energy_profile: Arc::new(RwLock::new(energy_profile)),
        };
        Ok(this)
    }

    pub async fn run_forever(self, interval: Duration) -> Result {
        let mut interval = tokio::time::interval(interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            self.run_once().await.context("the logger iteration has failed")?;
        }
    }

    pub async fn run_once(&self) -> Result {
        let (battery_metrics, grid_metrics) = (async || self.read_metrics().await)
            .retry(Self::BACKOFF)
            .notify(log_retried_error)
            .await?;

        let net_deficit = grid_metrics.active_power + battery_metrics.untracked.active_power;
        let balance = energy::Balance::new(self.battery_power_limits, net_deficit);
        debug!(
            ?net_deficit,
            battery.active_power = ?battery_metrics.untracked.active_power,
            battery.eps_active_power = ?battery_metrics.untracked.eps_active_power,
            battery.residual_energy = ?battery_metrics.tracked.residual_energy(),
            ?balance.battery.export,
            ?balance.battery.import,
            ?balance.grid.export,
            ?balance.grid.import,
            "measurements",
        );

        if self.update_energy_profile(balance, battery_metrics).await? {
            // TODO: optimize.
        }
        Ok(())
    }

    /// Read the MiniQube and HomeWizard P1 metrics simultaneously.
    async fn read_metrics(&self) -> Result<(mini_qube::Metrics, homewizard::EnergyMetrics)> {
        try_join!(
            async {
                self.connections
                    .battery
                    .read_metrics()
                    .await
                    .context("failed to read the battery metrics")
            },
            async {
                self.connections
                    .grid_measurement
                    .get_measurement()
                    .await
                    .context("failed to retrieve the grid measurement")
            }
        )
    }

    /// Track the balance and battery metrics and update the persistent energy profile.
    async fn update_energy_profile(
        &self,
        balance: energy::Balance<Watts>,
        battery_metrics: mini_qube::Metrics,
    ) -> Result<bool> {
        let mut energy_profile = self.energy_profile.write().await;
        energy_profile.update_energy_balance(
            balance,
            battery_metrics.untracked.eps_active_power,
            Local::now(),
            self.energy_balance_half_life,
        );
        let is_residual_energy_changed = energy_profile.track_battery_metrics(
            battery_metrics.tracked,
            self.battery_efficiency_half_life_factor,
        );
        energy_profile.write_to_file().await.context("failed to write the energy profile")?;
        drop(energy_profile);
        Ok(is_residual_energy_changed)
    }
}
