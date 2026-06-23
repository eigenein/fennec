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
use chrono::{DateTime, Days, Local, TimeDelta};
use clap::{Parser, crate_name, crate_version};
use fennec_modbus::contrib;
use sentry::{
    SessionMode,
    integrations::{anyhow::capture_anyhow, tracing::EventFilter},
};
use tokio::{spawn, sync::RwLock, time::MissedTickBehavior, try_join};
use tracing::metadata::LevelFilter;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

pub use self::schedule::Schedule;
use crate::{
    api::homewizard,
    cli::{Args, BatteryArgs, BatteryPowerLimits, Connections},
    energy::Flow,
    math::smoothing::HalfLife,
    prelude::*,
    quantity::{
        energy::{EnergyLevel, WattHours},
        power::Watts,
        price::KilowattHourPrice,
        time::Hours,
    },
    solution::{Optimizer, Step},
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
    let engine = Engine::start(&args).await?;
    let state = engine.state.clone();
    let engine_future = async { spawn(engine.run_forever(args.interval.into())).await? };
    let web_future = async { spawn(web::serve(args.bind.address, args.bind.port, state)).await? };
    try_join!(engine_future, web_future)?;
    Ok(())
}

#[must_use]
pub struct State {
    /// Current energy profile.
    energy_profile: energy::Profile,

    optimizer: Option<OptimizerState>,
}

#[must_use]
pub struct OptimizerState {
    metrics: solution::Metrics,
    steps: Schedule<(Flow<KilowattHourPrice>, Step)>,
}

/// TODO: store [`Args`] directly.
#[must_use]
pub struct Engine {
    /// API connections.
    connections: Connections,

    battery_power_limits: BatteryPowerLimits,
    energy_balance_half_life: HalfLife<Hours>,
    battery_efficiency_half_life_factor: f64,
    energy_provider: energy::Provider,
    dry_run: bool,
    battery_args: BatteryArgs,

    /// Current energy prices.
    energy_prices: Schedule<Flow<KilowattHourPrice>>,

    state: Arc<RwLock<State>>,
}

impl Engine {
    const BACKOFF: ConstantBuilder = ConstantBuilder::new().with_delay(Duration::from_secs(1));

    /// TODO: consume [`Args`].
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
            dry_run: args.dry_run,
            battery_args: args.battery.clone(), // TODO: kill `clone()`.
            energy_prices: Self::get_prices(args.energy_provider, Local::now()).await?,
            energy_provider: args.energy_provider,
            state: Arc::new(RwLock::new(State { energy_profile, optimizer: None })),
        };
        Ok(this)
    }

    pub async fn run_forever(mut self, interval: Duration) -> Result {
        let mut interval = tokio::time::interval(interval);
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
        let balance = energy::Balance::new(self.battery_power_limits, net_deficit);
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
                self.energy_prices = Self::get_prices(self.energy_provider, now).await?;
                // TODO: figure out whether the new prices came in.
            }
            self.optimize(&battery_metrics).await?;
        }

        Ok(())
    }

    /// Read the MiniQube and HomeWizard P1 metrics simultaneously.
    async fn read_metrics(&self) -> Result<(api::mini_qube::Metrics, homewizard::EnergyMetrics)> {
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
    ) -> Result<Schedule<Flow<KilowattHourPrice>>> {
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
        battery_metrics: &api::mini_qube::Metrics,
    ) -> Result<bool> {
        let energy_profile = &mut self.state.write().await.energy_profile;
        energy_profile.update_energy_balance(
            balance,
            battery_metrics.untracked.eps_active_power,
            now,
            self.energy_balance_half_life,
        );
        let is_residual_energy_changed = energy_profile.track_battery_metrics(
            battery_metrics.tracked,
            self.battery_efficiency_half_life_factor,
        );
        energy_profile.write_to_file().await.context("failed to write the energy profile")?;
        Ok(is_residual_energy_changed)
    }

    #[instrument(skip_all)]
    async fn optimize(&self, battery_metrics: &api::mini_qube::Metrics) -> Result {
        let state = self.state.read().await;

        let min_energy_level = EnergyLevel::from(battery_metrics.min_residual_charge());
        let max_energy_level = EnergyLevel::from(battery_metrics.max_residual_charge());
        let initial_energy_level =
            WattHours::from(battery_metrics.tracked.residual_energy()).into();
        let (metrics, steps) = Optimizer::builder()
            .working_modes(self.battery_args.working_modes.iter().copied().collect())
            .allowed_energy_levels(min_energy_level..=max_energy_level)
            .battery_efficiency(state.energy_profile.battery_efficiency)
            .battery_capacity(battery_metrics.tracked.actual_capacity())
            .max_battery_flow(
                self.battery_args
                    .power_limits
                    .max_effective_flow(state.energy_profile.eps_active_power.0),
            )
            .energy_profile(&state.energy_profile)
            .battery_degradation_cost(self.battery_args.degradation_cost)
            .build()
            .solve(&self.energy_prices) // FIXME: `spawn_blocking`.
            .solutions
            .backtrack(initial_energy_level)?;

        drop(state); // TODO: accept from outside?
        info!(
            grid_loss = ?metrics.losses.grid,
            battery.loss = ?metrics.losses.battery,
            battery.charge = ?metrics.internal_battery_flow.import,
            battery.discharge = ?metrics.internal_battery_flow.export,
            "solution summary",
        );

        let schedule = api::mini_qube::schedule::build(
            steps.iter().map(|slot| (slot.interval, slot.value.1.working_mode)),
            battery_metrics.untracked.allowed_charge,
            self.battery_args.power_limits,
        );
        self.write_schedule(&schedule).await?;

        self.state.write().await.optimizer = Some(OptimizerState { metrics, steps });
        Ok(())
    }

    /// Write the battery schedule.
    ///
    /// On dry run, print out the schedule without pushing it to the battery.
    async fn write_schedule(&self, schedule: &contrib::mini_qube::schedule::Full) -> Result {
        if self.dry_run {
            warn!("not writing the schedule to the battery, just scouting");
            for entry in schedule {
                info!(?entry.start_time, ?entry.end_time, ?entry.working_mode);
            }
        } else {
            (async || self.connections.battery.write_schedule(schedule).await)
                .retry(Self::BACKOFF)
                .notify(log_retried_error)
                .await
                .context("failed to push the schedule to the battery")?;
        }
        Ok(())
    }
}
