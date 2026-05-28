use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use backon::{ConstantBuilder, Retryable};
use chrono::Local;
use tokio::try_join;

use crate::{
    cli::{battery, connection::Connections, state},
    cron::CronSchedule,
    db::{Measurement, power},
    energy,
    energy::Balance,
    math::smoothing::HalfLife,
    prelude::*,
};

/// Battery state and power meter logger.
pub struct Logger {
    connections: Connections,
    battery_power_limits: battery::PowerLimits,
    energy_profile: energy::Profile,
    energy_profile_half_life: HalfLife,
}

impl Logger {
    const BACKOFF: ConstantBuilder = ConstantBuilder::new().with_delay(Duration::from_secs(1));

    pub async fn new(
        connections: Connections,
        battery_power_limits: battery::PowerLimits,
        energy_profile_half_life: HalfLife,
    ) -> Result<Self> {
        let energy_profile = energy::Profile::read_or_default().await?;
        Ok(Self { connections, battery_power_limits, energy_profile, energy_profile_half_life })
    }

    pub async fn run_forever(
        mut self,
        schedule: CronSchedule,
        state: Arc<RwLock<state::Logger>>,
    ) -> Result {
        let mut cron = schedule.start();
        loop {
            cron.wait_until_next().await?;
            *state.write().unwrap() =
                self.run_once().await.context("the logger iteration has failed")?;
        }
    }

    /// Run a single logging iteration.
    pub async fn run_once(&mut self) -> Result<state::Logger> {
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
        let balance = Balance::new(self.battery_power_limits, net_deficit);
        info!(
            grid.import = ?balance.grid.import,
            grid.export = ?balance.grid.export,
            battery.import = ?balance.battery.import,
            battery.export = ?balance.battery.export,
            "energy balance",
        );

        self.energy_profile.update(
            balance,
            battery_state.eps_active_power,
            Local::now(),
            self.energy_profile_half_life,
        );
        self.energy_profile.write().await?;

        let battery_measurement = power::BatteryMeasurement::builder()
            .residual_energy(battery_state.residual_energy_watt_hours())
            .active_power(battery_state.active_power)
            .eps_active_power(battery_state.eps_active_power)
            .build();
        power::Measurement::builder()
            .battery(battery_measurement)
            .build()
            .insert_into(&self.connections.db)
            .await?;
        info!(
            grid.net_deficit = ?net_deficit,
            battery.active_power = ?battery_measurement.active_power,
            battery.eps_active_power = ?battery_measurement.eps_active_power,
            battery.residual_energy = ?battery_measurement.residual_energy,
            "measurements",
        );
        info!(
            import = ?battery_state.total_grid_flow.import,
            export = ?battery_state.total_grid_flow.export,
            "total battery grid flow",
        );

        Ok(state::Logger { battery: battery_state, energy_profile: self.energy_profile.clone() })
    }
}
