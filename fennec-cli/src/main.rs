#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_panics_doc)]
#![doc = include_str!("../../README.md")]

mod api;
mod battery;
mod cli;
mod energy;
mod math;
mod ops;
mod prelude;
mod quantity;
mod schedule;
mod solution;
mod web;

use std::{borrow::Cow, range::RangeInclusive, sync::Arc, time::Duration};

use backon::{ConstantBuilder, Retryable};
use chrono::{DateTime, Days, Local, TimeDelta};
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
    cli::{Args, Connections, EngineArgs},
    prelude::*,
    quantity::{
        energy::{EnergyLevel, WattHours},
        power::Watts,
        price::KilowattHourPrice,
        ratios::Percentage,
    },
    solution::{Backtrack, Optimizer, Step},
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
    let engine = Engine::start(args.connections.connect()?, args.engine).await?;
    let state = engine.state.clone();
    let engine_future = async { spawn(engine.run_forever()).await? };
    let web_future = async { spawn(web::serve(args.bind.address, args.bind.port, state)).await? };
    try_join!(engine_future, web_future)?;
    Ok(())
}

#[must_use]
pub struct State {
    /// Current energy profile.
    energy_profile: energy::Profile,

    /// Current solution backtrack.
    backtrack: Option<Backtrack>,
}

/// TODO: store [`Args`] directly.
#[must_use]
pub struct Engine {
    /// API connections.
    connections: Connections,

    args: EngineArgs,

    /// Current energy prices.
    ///
    /// TODO: we'll have that in `steps`.
    energy_prices: Schedule<energy::Flow<KilowattHourPrice>>,

    state: Arc<RwLock<State>>,
}

impl Engine {
    const BACKOFF: ConstantBuilder = ConstantBuilder::new().with_delay(Duration::from_secs(1));

    #[instrument(skip_all)]
    pub async fn start(connections: Connections, args: EngineArgs) -> Result<Self> {
        let energy_profile =
            energy::Profile::read_from_file(args.energy_profile.n_balance_harmonics).await?;
        let energy_provider = args.energy_provider;
        let this = Self {
            connections,
            args,
            energy_prices: Self::get_prices(energy_provider, Local::now()).await?,
            state: Arc::new(RwLock::new(State { energy_profile, backtrack: None })),
        };
        Ok(this)
    }

    pub async fn run_forever(mut self) -> Result {
        let mut interval = tokio::time::interval(self.args.interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            self.run_once().await.context("the logger iteration has failed")?;
        }
    }

    pub async fn run_once(&mut self) -> Result {
        let now = Local::now();
        let (battery_metrics, grid_metrics) = (async || self.read_metrics().await)
            .retry(Self::BACKOFF)
            .notify(log_retried_error)
            .await?;

        let net_deficit = grid_metrics.active_power + battery_metrics.untracked.active_power;
        let balance = energy::Balance::new(self.args.battery.power_limits, net_deficit);
        debug!(
            ?net_deficit,
            battery.active_power = ?battery_metrics.untracked.active_power,
            battery.eps_active_power = ?battery_metrics.untracked.eps_active_power,
            battery.residual_energy = ?battery_metrics.tracked.residual_energy(),
            battery.health = ?battery_metrics.tracked.health,
            battery.actual_capacity = ?battery_metrics.tracked.actual_capacity(),
            ?balance.battery.export,
            ?balance.battery.import,
            ?balance.grid.export,
            ?balance.grid.import,
            "measurements",
        );

        let has_residual_charge_changed =
            self.update_energy_profile(now, balance, &battery_metrics).await?;
        let has_schedule_advanced = {
            let previous_len = self.energy_prices.len();
            self.energy_prices.advance_to(now);
            self.energy_prices.len() != previous_len
        };
        if has_residual_charge_changed || has_schedule_advanced {
            if self.energy_prices.duration() <= TimeDelta::hours(12) {
                // When the time comes, try to fetch the tomorrow prices:
                self.energy_prices = Self::get_prices(self.args.energy_provider, now).await?;
                // TODO: figure out whether the new prices came in.
            }
            let backtrack = self
                .reoptimize_schedule(
                    &battery_metrics,
                    self.state.read().await.energy_profile.clone(),
                )
                .await?;
            self.write_schedule(&backtrack.schedule, battery_metrics.untracked.allowed_charge)
                .await?;
            self.state.write().await.backtrack = Some(backtrack);
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

    /// Fetch energy prices for up to 2 days.
    #[instrument(skip_all, fields(now = ?now))]
    async fn get_prices(
        energy_provider: energy::Provider,
        now: DateTime<Local>,
    ) -> Result<Schedule<energy::Flow<KilowattHourPrice>>> {
        const ONE_DAY: Days = Days::new(1);

        // TODO: do not re-read prices for today:
        let today = now.date_naive();
        let mut prices = energy_provider.get_prices(today).await?;
        ensure!(prices.len() != 0, "received empty price schedule for today");

        prices.extend({
            let tomorrow = today.checked_add_days(ONE_DAY).unwrap();
            energy_provider.get_prices(tomorrow).await?
        })?;

        info!(len = prices.len(), "fetched energy prices");
        prices.advance_to(now);
        Ok(prices)
    }

    /// Track the balance and battery metrics and update the persistent energy profile.
    async fn update_energy_profile(
        &self,
        now: DateTime<Local>,
        balance: energy::Balance<Watts>,
        battery_metrics: &mini_qube::Metrics,
    ) -> Result<bool> {
        let energy_profile = &mut self.state.write().await.energy_profile;
        energy_profile.update_energy_balance(
            balance,
            battery_metrics.untracked.eps_active_power,
            now,
            self.args.energy_profile.balance_half_life,
        );
        let is_residual_energy_changed = energy_profile.track_battery_metrics(
            battery_metrics.tracked,
            self.args.energy_profile.battery_efficiency_half_life_factor,
        );
        energy_profile.write_to_file().await.context("failed to write the energy profile")?;
        Ok(is_residual_energy_changed)
    }

    /// Fully re-optimize the battery schedule.
    #[instrument(skip_all)]
    async fn reoptimize_schedule(
        &self,
        battery_metrics: &mini_qube::Metrics,
        energy_profile: energy::Profile,
    ) -> Result<Backtrack> {
        let min_energy_level = EnergyLevel::from(battery_metrics.min_residual_charge());
        let max_energy_level = EnergyLevel::from(battery_metrics.max_residual_charge());
        let initial_energy_level =
            WattHours::from(battery_metrics.tracked.residual_energy()).into();
        let backtrack = Optimizer::builder()
            .battery_args(self.args.battery.clone()) // FIXME: cloning.
            .allowed_energy_levels(min_energy_level..=max_energy_level)
            .battery_capacity(battery_metrics.tracked.actual_capacity())
            .max_battery_flow(
                self.args
                    .battery
                    .power_limits
                    .max_effective_flow(energy_profile.eps_active_power.0),
            )
            .energy_profile(energy_profile)
            .build()
            .solve(&self.energy_prices) // TODO: consume energy prices.
            .solutions
            .backtrack(initial_energy_level)?;

        info!(
            grid_loss = ?backtrack.metrics.losses.grid,
            battery.loss = ?backtrack.metrics.losses.battery,
            battery.charge = ?backtrack.metrics.internal_battery_flow.import,
            battery.discharge = ?backtrack.metrics.internal_battery_flow.export,
            "solution summary",
        );
        Ok(backtrack)
    }

    /// Write the battery schedule.
    ///
    /// On dry run, print out the schedule without pushing it to the battery.
    async fn write_schedule(
        &self,
        schedule: &Schedule<(energy::Flow<KilowattHourPrice>, Step)>,
        allowed_charge: RangeInclusive<Percentage>,
    ) -> Result {
        let schedule = mini_qube::schedule::build(
            schedule.iter().map(|slot| (slot.interval, slot.value.1.working_mode)),
            allowed_charge,
            self.args.battery.power_limits,
        );
        if self.args.dry_run {
            warn!("not writing the schedule to the battery, just scouting");
            for entry in schedule {
                info!(?entry.start_time, ?entry.end_time, ?entry.working_mode);
            }
        } else {
            (async || self.connections.battery.write_schedule(&schedule).await)
                .retry(Self::BACKOFF)
                .notify(log_retried_error)
                .await
                .context("failed to push the schedule to the battery")?;
        }
        Ok(())
    }
}
