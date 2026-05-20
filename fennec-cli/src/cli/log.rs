use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use backon::{ConstantBuilder, Retryable};
use chrono::Local;
use tokio::try_join;

use crate::{
    cli::{battery, connection::Connections},
    cron::CronSchedule,
    db::{Measurement, power},
    energy,
    energy::Balance,
    ops::smoothing::HalfLife,
    prelude::*,
    state::LoggerState,
};

/// Battery state and power meter logger.
pub struct Logger {
    connections: Connections,
    battery_power_limits: battery::PowerLimits,
    energy_profile: energy::ExponentialProfile,
    energy_profile_decay: HalfLife,
}

impl Logger {
    const BACKOFF: ConstantBuilder = ConstantBuilder::new().with_delay(Duration::from_secs(1));

    pub async fn new(
        connections: Connections,
        battery_power_limits: battery::PowerLimits,
        energy_profile_decay: HalfLife,
    ) -> Result<Self> {
        let energy_profile = energy::ExponentialProfile::read_or_default().await?;
        Ok(Self { connections, battery_power_limits, energy_profile, energy_profile_decay })
    }

    pub async fn run_forever(
        mut self,
        schedule: CronSchedule,
        state: Arc<RwLock<LoggerState>>,
    ) -> Result {
        let mut cron = schedule.start();
        loop {
            cron.wait_until_next().await?;
            *state.write().unwrap() =
                self.run_once().await.context("the logger iteration has failed")?;
        }
    }

    /// Run a single logging iteration.
    pub async fn run_once(&mut self) -> Result<LoggerState> {
        let read_state = || async {
            // Retry them together to ensure the measurements are in sync.
            try_join!(
                async { self.connections.battery.read_state().await },
                self.connections.grid_measurement.get_measurement()
            )
        };
        let (battery_state, grid_metrics) = read_state
            .retry(Self::BACKOFF)
            .notify(log_retried_error)
            .await
            .context("failed to read the energy state")?;

        let net_deficit = grid_metrics.active_power + battery_state.active_power;
        self.energy_profile
            .update(
                Balance::new(self.battery_power_limits, net_deficit),
                Local::now(),
                self.energy_profile_decay,
            )
            .write()
            .await?;

        let battery_measurement = power::BatteryMeasurement::builder()
            .residual_energy(battery_state.residual_energy())
            .active_power(battery_state.active_power)
            .eps_active_power(battery_state.eps_active_power)
            .build();
        power::Measurement::builder()
            .net_deficit(net_deficit)
            .battery(battery_measurement)
            .build()
            .insert_into(&self.connections.db)
            .await?;

        Ok(LoggerState { battery: battery_state })
    }
}
