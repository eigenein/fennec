use std::{sync::Arc, time::Duration};

use backon::{ConstantBuilder, Retryable};
use bon::Builder;
use chrono::Local;
use tokio::{sync::RwLock, try_join};

use crate::{
    api::{homewizard, mini_qube},
    cli::{BatteryPowerLimits, Connections},
    cron::CronSchedule,
    energy,
    energy::Balance,
    math::smoothing::HalfLife,
    prelude::*,
    quantity::time::Hours,
};

#[must_use]
#[derive(Clone, Builder)]
pub struct Args {
    connections: Connections,
    battery_power_limits: BatteryPowerLimits,
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
#[must_use]
#[derive(Clone)]
pub struct Runner {
    args: Args,
    pub energy_profile: Arc<RwLock<energy::Profile>>,
}

impl Runner {
    const BACKOFF: ConstantBuilder = ConstantBuilder::new().with_delay(Duration::from_secs(1));

    pub async fn run_forever(self, schedule: CronSchedule) -> Result {
        let mut cron = schedule.start();
        loop {
            cron.wait_until_next().await?;
            self.run_once().await.context("the logger iteration has failed")?;
        }
    }

    pub async fn run_once(&self) -> Result {
        let (battery_metrics, grid_metrics) = (async || self.read_metrics().await)
            .retry(Self::BACKOFF)
            .notify(log_retried_error)
            .await?;

        let net_deficit = grid_metrics.active_power + battery_metrics.untracked.active_power;
        let balance = Balance::new(self.args.battery_power_limits, net_deficit);
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

        let mut energy_profile = self.energy_profile.write().await;
        energy_profile.update_energy_balance(
            balance,
            battery_metrics.untracked.eps_active_power,
            Local::now(),
            self.args.energy_balance_half_life,
        );
        energy_profile.update_battery_metrics(
            battery_metrics.tracked,
            self.args.battery_efficiency_half_life_factor,
        );
        energy_profile.write_to_file().await.context("failed to write the energy profile")?;
        drop(energy_profile);

        Ok(())
    }

    /// Read the MiniQube and HomeWizard P1 metrics simultaneously.
    async fn read_metrics(&self) -> Result<(mini_qube::Metrics, homewizard::EnergyMetrics)> {
        try_join!(
            async {
                self.args
                    .connections
                    .battery
                    .read_metrics()
                    .await
                    .context("failed to read the battery metrics")
            },
            async {
                self.args
                    .connections
                    .grid_measurement
                    .get_measurement()
                    .await
                    .context("failed to retrieve the grid measurement")
            }
        )
    }
}
