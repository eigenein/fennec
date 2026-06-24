use std::{range::RangeInclusive, sync::Arc, time::Duration};

use backon::{ConstantBuilder, Retryable};
use chrono::{DateTime, Days, Local, TimeDelta};
use tokio::{sync::RwLock, time::MissedTickBehavior, try_join};

use crate::{
    Schedule,
    api::{Connections, homewizard, mini_qube},
    cli::EngineArgs,
    energy,
    prelude::*,
    quantity::{energy::WattHours, power::Watts, price::KilowattHourPrice, ratios::Percentage},
    solution::{Backtrack, Optimizer, Step},
};

#[must_use]
pub struct State {
    /// Current energy profile.
    pub energy_profile: energy::Profile,

    /// Current solution backtrack.
    pub backtrack: Option<Backtrack>,
}

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

    pub fn state(&self) -> Arc<RwLock<State>> {
        self.state.clone()
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

        let net_deficit = grid_metrics.active_power + battery_metrics.active_power;
        let balance = energy::Balance::new(self.args.battery.power_limits, net_deficit);
        debug!(
            ?net_deficit,
            battery.active_power = ?battery_metrics.active_power,
            battery.eps_active_power = ?battery_metrics.eps_active_power,
            battery.residual_energy = ?battery_metrics.residual_energy(),
            battery.state_of_health = ?battery_metrics.state_of_health,
            battery.actual_capacity = ?battery_metrics.actual_capacity(),
            ?balance.battery.export,
            ?balance.battery.import,
            ?balance.grid.export,
            ?balance.grid.import,
            "measurements",
        );

        let has_residual_charge_changed =
            // TODO: must also react on min-max SoC settings.
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
            let optimizer = Optimizer::new(
                self.state.read().await.energy_profile.clone(),
                &self.args.battery,
                &battery_metrics,
            );
            let solutions = optimizer.solve(&self.energy_prices); // TODO: consume energy prices.
            let backtrack = {
                let initial_energy_level =
                    WattHours::from(battery_metrics.residual_energy()).into();
                solutions.backtrack(initial_energy_level)?
            };
            info!(
                grid_loss = ?backtrack.metrics.losses.grid,
                battery.loss = ?backtrack.metrics.losses.battery,
                battery.charge = ?backtrack.metrics.internal_battery_flow.import,
                battery.discharge = ?backtrack.metrics.internal_battery_flow.export,
                "solution summary",
            );
            self.write_schedule(&backtrack.schedule, battery_metrics.allowed_soc).await?;
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
        energy_profile.balance.update(
            balance,
            battery_metrics.eps_active_power,
            now,
            self.args.energy_profile.balance_half_life,
        );
        let is_residual_energy_changed = energy_profile
            .battery
            .track(battery_metrics, self.args.energy_profile.battery_efficiency_half_life_factor);
        energy_profile.write_to_file().await.context("failed to write the energy profile")?;
        Ok(is_residual_energy_changed)
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
