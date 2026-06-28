use std::{range::RangeInclusive, sync::Arc, time::Duration};

use backon::{ConstantBuilder, Retryable};
use chrono::{DateTime, Local};
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

        let has_solution_space_advanced = self.optimizer.as_mut().is_none_or(|optimizer| {
            let space_len = optimizer.solution_space().len();
            optimizer.solution_space().advance_to(now);
            optimizer.solution_space().len() != space_len
        });
        let has_residual_energy_changed =
            self.update_energy_profile(now, balance, &battery_metrics).await?;
        // TODO: if has_solution_space_advanced and (solution_space.duration() <= TimeDelta::hours(12)):
        //           # fetch the prices
        //           # are_prices_updated = ...
        //       else:
        //           # are_prices_updated = False
        // TODO: if are_prices_updated or has_allowed_energy_levels_changed or has_actual_capacity_changed:
        //           # fully re-solve
        //       elif has_solution_space_advanced or has_residual_energy_changed:
        //           # re-optimize the state
        if has_residual_energy_changed || has_solution_space_advanced {
            let energy_prices = self.args.energy_provider.get_future_prices(now).await?;
            let mut optimizer = Optimizer::new(
                self.state.read().await.energy_profile.clone(),
                &self.args.battery,
                battery_metrics.actual_capacity(),
                battery_metrics.allowed_energy_levels(),
            );
            optimizer.solve(&energy_prices);
            let plan = {
                let initial_energy_level =
                    EnergyLevel::from(WattHours::from(battery_metrics.residual_energy()));
                optimizer.solution_space().backtrack(initial_energy_level)?
            };
            self.optimizer = Some(optimizer);
            info!(
                grid_loss = ?plan.metrics.losses.grid,
                battery.loss = ?plan.metrics.losses.battery,
                battery.charge = ?plan.metrics.internal_battery_flow.import,
                battery.discharge = ?plan.metrics.internal_battery_flow.export,
                "solution summary",
            );
            self.write_schedule(&plan.schedule, battery_metrics.allowed_soc).await?;
            self.state.write().await.plan = Some(plan);
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
