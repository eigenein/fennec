use std::{sync::Arc, time::Duration};

use backon::{ConstantBuilder, Retryable};
use bon::Builder;
use chrono::Local;
use tokio::{
    sync::{RwLock, RwLockReadGuard},
    try_join,
};

use crate::{
    cli::{battery, connection::Connections},
    cron::CronSchedule,
    energy,
    energy::Balance,
    math::smoothing::HalfLife,
    prelude::*,
    quantity::{energy::WattHours, time::Hours},
};

#[must_use]
#[derive(Clone, Builder)]
pub struct Args {
    connections: Connections,
    battery_power_limits: battery::PowerLimits,
    energy_balance_half_life: HalfLife<Hours>,
    battery_efficiency_half_life_factor: f64,
    n_balance_harmonics: usize,
}

impl Args {
    #[instrument(skip_all)]
    pub async fn start(self) -> Result<Runner> {
        let energy_profile = energy::Profile::read_from_file(self.n_balance_harmonics).await?;
        Ok(Runner { args: self, energy_profile: Arc::new(RwLock::new(energy_profile)) })
    }
}

/// Battery state and power meter logger.
///
/// TODO: rename to `Runner`.
#[must_use]
#[derive(Clone)]
pub struct Runner {
    args: Args,
    energy_profile: Arc<RwLock<energy::Profile>>,
}

impl Runner {
    const BACKOFF: ConstantBuilder = ConstantBuilder::new().with_delay(Duration::from_secs(1));

    pub async fn energy_profile(&self) -> RwLockReadGuard<'_, energy::Profile> {
        self.energy_profile.read().await
    }

    pub async fn run_forever(self, schedule: CronSchedule) -> Result {
        let mut cron = schedule.start();
        loop {
            cron.wait_until_next().await?;
            self.run_once().await.context("the logger iteration has failed")?;
        }
    }

    pub async fn run_once(&self) -> Result {
        let read_state = || async {
            // Retry them together to ensure the measurements are in sync.
            try_join!(
                async { self.args.connections.battery.read_state().await },
                self.args.connections.grid_measurement.get_measurement()
            )
        };
        let (battery_metrics, grid_metrics) = read_state
            .retry(Self::BACKOFF)
            .notify(log_retried_error)
            .await
            .context("failed to read the energy state")?;

        let net_deficit = grid_metrics.active_power + battery_metrics.active_power;
        let balance = Balance::new(self.args.battery_power_limits, net_deficit);
        info!(
            grid.import = ?balance.grid.import,
            grid.export = ?balance.grid.export,
            battery.import = ?balance.battery.import,
            battery.export = ?balance.battery.export,
            "energy balance",
        );
        info!(
            grid.net_deficit = ?net_deficit,
            battery.active_power = ?battery_metrics.active_power,
            battery.eps_active_power = ?battery_metrics.eps_active_power,
            battery.residual_energy = ?WattHours::from(battery_metrics.residual_energy()),
            "measurements",
        );

        let mut energy_profile = self.energy_profile.write().await;
        energy_profile.update_energy_balance(
            balance,
            battery_metrics.eps_active_power,
            Local::now(),
            self.args.energy_balance_half_life,
        );
        energy_profile
            .update_battery_metrics(battery_metrics, self.args.battery_efficiency_half_life_factor);
        energy_profile.write_to_file().await.context("failed to write the energy profile")?;
        drop(energy_profile);

        Ok(())
    }
}
