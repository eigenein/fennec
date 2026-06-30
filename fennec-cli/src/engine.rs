use std::{range::RangeInclusive, sync::Arc, time::Duration};

use backon::{ConstantBuilder, Retryable};
use chrono::{DateTime, Local, TimeDelta};
use fennec_modbus::contrib;
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

        let initial_energy_level =
            EnergyLevel::from(WattHours::from(battery_metrics.residual_energy()));
        let battery_capacity = battery_metrics.actual_capacity();
        let allowed_energy_levels = battery_metrics.allowed_energy_levels();

        // Invalidate the optimizer if the critical parameters have changed:
        if self
            .optimizer
            .take_if(|optimizer| optimizer.is_stale(battery_capacity, allowed_energy_levels))
            .is_some()
        {
            info!("optimizer invalidated: battery parameters changed");
        }

        // Update the energy profile before we potentially fail to optimize:
        let has_residual_energy_changed =
            self.update_energy_profile(now, balance, &battery_metrics).await?;

        let mut has_solution_space_advanced = false;

        // Abandon hope, all ye who enter here – every tick, we decide the optimizer's fate:
        let decision = match self.optimizer.take() {
            None => {
                // The battery parameters have changed or the cold start:
                let prices = self.args.energy_provider.get_future_prices(now).await?;
                Decision::Prices(prices)
            }
            Some(mut optimizer) => {
                has_solution_space_advanced = optimizer.advance_to(now);
                if optimizer.solution_space().duration() <= TimeDelta::hours(12) {
                    // The horizon is too short; fetch prices to see if tomorrow's data has arrived:
                    let prices = self.args.energy_provider.get_future_prices(now).await?;
                    if prices.end() == optimizer.solution_space().end() {
                        // The end is the same and realistically the prices never change mid-period,
                        // so continue with the current optimizer:
                        Decision::Optimizer(optimizer)
                    } else {
                        // New prices arrived; the existing solution space is now too short. Rebuild:
                        info!("optimizer invalidated: new prices arrived");
                        Decision::Prices(prices)
                    }
                } else {
                    // The horizon is long enough, just continue with the current optimizer:
                    Decision::Optimizer(optimizer)
                }
            }
        };

        // Now that we have the decision, we have to execute it:
        let optimizer = match decision {
            Decision::Prices(prices) => {
                let mut optimizer = Optimizer::new(
                    self.state.read().await.energy_profile.clone(),
                    &self.args.battery,
                    battery_capacity,
                    allowed_energy_levels,
                );
                optimizer.solve(&prices);
                optimizer
            }
            Decision::Optimizer(mut optimizer)
                if has_solution_space_advanced || has_residual_energy_changed =>
            {
                // No need to fully solve: re-optimizing interval 0 adjusts for the current battery level while
                // reusing the pre-computed future solutions. The energy profile can stay stale here – if it has
                // changed significantly, the solution space will eventually be rebuilt when prices refresh.
                info!(?initial_energy_level, "optimizing the current state");
                optimizer.optimize_state(0, initial_energy_level);
                optimizer
            }
            Decision::Optimizer(optimizer) => {
                // The most frequent case: both the optimizer and the plan stay.
                self.optimizer = Some(optimizer);
                return Ok(());
            }
        };

        // Done, extract the plan and push it to the battery:
        let plan = optimizer
            .solution_space()
            .backtrack(initial_energy_level)
            .inspect(Plan::trace_summary)?;
        self.write_schedule(&plan.schedule, battery_metrics.allowed_soc).await?;

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

    /// Write the battery schedule.
    ///
    /// On dry run, print out the schedule without pushing it to the battery.
    async fn write_schedule(
        &self,
        schedule: &Schedule<(energy::Flow<KilowattHourPrice>, Step)>,
        allowed_soc: RangeInclusive<Percentage>,
    ) -> Result {
        if self.args.dry_run {
            warn!("not writing the schedule to the battery, just scouting");
            return Ok(());
        }
        for slot in schedule.iter().take(contrib::mini_qube::schedule::Entry::N_TOTAL) {
            let index = mini_qube::schedule::index_of(slot.interval);
            let entry = mini_qube::schedule::make_entry(
                index,
                slot.value.1.working_mode,
                allowed_soc,
                self.args.battery.power_limits,
            );
            (async || self.connections.battery.write_schedule_entry(index.into(), entry).await)
                .retry(Self::BACKOFF)
                .notify(log_retried_error)
                .await
                .context("failed to push the schedule to the battery")?;
        }
        info!("done");
        Ok(())
    }
}

/// Decision at each iteration of the [`Engine`] loop: survives either [`Optimizer`],
/// or pricing schedule for full [`Optimizer`] reconstruction.
#[must_use]
enum Decision {
    /// Keep the current [`Optimizer`].
    Optimizer(Optimizer),

    /// Drop the [`Optimizer`], construct a new one and optimize for the prices.
    Prices(Schedule<energy::Flow<KilowattHourPrice>>),
}
