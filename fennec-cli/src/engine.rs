use std::{range::RangeInclusive, sync::Arc, time::Duration};

use backon::{ConstantBuilder, Retryable};
use chrono::{DateTime, Local, TimeDelta};
use tokio::{sync::RwLock, time::MissedTickBehavior, try_join};

use crate::{
    Schedule,
    api::{Connections, homewizard, mini_qube},
    cli::EngineArgs,
    energy,
    prelude::*,
    quantity::{
        energy::{EnergyLevel, WattHours},
        power::Watts,
        price::KilowattHourPrice,
        ratios::Percentage,
    },
    series::Slot,
    solution::{Optimizer, Plan, Step},
};

#[must_use]
pub struct State {
    /// Current energy profile.
    pub energy_profile: energy::Profile,

    /// Current solution backtrack.
    pub plan: Option<Plan>,
}

#[must_use]
pub struct Engine {
    connections: Connections,
    args: EngineArgs,
    state: Arc<RwLock<State>>,
    optimizer: Option<Optimizer>,
}

impl Engine {
    const BACKOFF: ConstantBuilder = ConstantBuilder::new().with_delay(Duration::from_secs(1));

    #[instrument(skip_all)]
    pub async fn start(connections: Connections, args: EngineArgs) -> Result<Self> {
        let energy_profile =
            energy::Profile::read_from_file(args.energy_profile.n_balance_harmonics).await?;
        let this = Self {
            connections,
            args,
            state: Arc::new(RwLock::new(State { energy_profile, plan: None })),
            optimizer: None,
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
            self.run_once().await?;
            self.connections.heartbeat.send().await;
        }
    }

    /// Run a single engine iteration.
    ///
    /// Note that we *only write at most the upcoming battery slot* to the battery. Motivation:
    ///
    /// - Potential Flash/EEPROM wear on the battery when writing all the changed slots every time.
    /// - Finite horizon problem: e.g. writing tomorrow afternoon barely makes sense
    ///   before the future prices become known.
    /// - Flaky future slots due to small oscillations in the Fourier decomposition.
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

        let initial_energy_level =
            EnergyLevel::from(WattHours::from(battery_metrics.residual_energy()));
        let battery_capacity = battery_metrics.actual_capacity();
        let allowed_energy_levels = battery_metrics.allowed_energy_levels();

        let has_residual_energy_changed =
            self.update_energy_profile(now, balance, &battery_metrics).await?;

        let optimizer = match self.optimizer.take() {
            Some(mut optimizer) if optimizer.matches(battery_capacity, allowed_energy_levels) => {
                let (has_solution_space_advanced, first_interval) = optimizer.advance_to(now);
                if (
                    // Nothing happened in the meantime:
                    !has_solution_space_advanced && !has_residual_energy_changed
                ) || (
                    // The interval is getting too short to re-optimize the state:
                    // FIXME: `unwrap`.
                    // TODO: make the deadline configurable.
                    first_interval.unwrap().duration() < TimeDelta::minutes(1)
                ) {
                    self.optimizer = Some(optimizer);
                    return Ok(());
                }

                let new_prices = if optimizer.solution_space().duration() <= TimeDelta::hours(12) {
                    // Try to extend the price horizon if it's getting short:
                    let prices = self.args.energy_provider.get_future_prices(now).await?;
                    (prices.end() != optimizer.solution_space().end()).then_some(prices)
                } else {
                    None
                };

                if let Some(prices) = new_prices {
                    info!("optimizer invalidated: new prices arrived");
                    self.rebuild_optimizer(&prices, battery_capacity, allowed_energy_levels).await
                } else {
                    info!(?initial_energy_level, "optimizing current state");
                    optimizer.optimize_state(0, initial_energy_level);
                    optimizer
                }
            }

            stale_optimizer => {
                if stale_optimizer.is_some() {
                    info!("optimizer invalidated: battery parameters changed");
                } else {
                    info!("initializing optimizer: cold start");
                }
                let prices = self.args.energy_provider.get_future_prices(now).await?;
                self.rebuild_optimizer(&prices, battery_capacity, allowed_energy_levels).await
            }
        };

        let plan = optimizer
            .solution_space()
            .backtrack(initial_energy_level)
            .inspect(|plan| plan.trace_summary(battery_metrics.design_capacity))?;
        // TODO: potential improvement – make the number of written slots configurable:
        let slot = plan.schedule.get(0);
        let working_mode = slot.value.1.working_mode;
        if self.args.dry_run {
            warn!("not writing the schedule to the battery, just scouting");
        } else {
            self.write_schedule_slot(slot, battery_metrics.allowed_soc).await?;
            self.connections.home_assistant_working_mode.post(&format!("{working_mode:?}")).await;
        }

        // Commit the new state:
        self.state.write().await.plan = Some(plan);
        self.optimizer = Some(optimizer);

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

    /// Rebuild [`Optimizer`] from scratch.
    async fn rebuild_optimizer(
        &self,
        prices: &Schedule<energy::Flow<KilowattHourPrice>>,
        battery_capacity: WattHours,
        allowed_energy_levels: RangeInclusive<EnergyLevel>,
    ) -> Optimizer {
        let mut optimizer = Optimizer::new(
            self.state.read().await.energy_profile.clone(),
            &self.args.battery,
            battery_capacity,
            allowed_energy_levels,
        );
        optimizer.solve(prices);
        optimizer
    }

    /// Write the schedule slot to the battery, if not dry run.
    async fn write_schedule_slot(
        &self,
        slot: Slot<&(energy::Flow<KilowattHourPrice>, Step)>,
        allowed_soc: RangeInclusive<Percentage>,
    ) -> Result {
        let index = mini_qube::schedule::index_of(slot.interval);
        let slot = mini_qube::schedule::make_slot(
            index,
            slot.value.1.working_mode,
            allowed_soc,
            self.args.battery.power_limits,
        );
        (|| async { self.connections.battery.write_schedule_slot(index, slot).await })
            .retry(Self::BACKOFF)
            .notify(log_retried_error)
            .await
            .with_context(|| {
                format!("failed to write the schedule slot #{index} to the battery")
            })?;

        Ok(())
    }
}
