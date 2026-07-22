use std::path::Path;

use chrono::{DateTime, Local, NaiveTime};
use musli::{Decode, Encode, wire};

use crate::{
    api::mini_qube,
    energy,
    math::{
        fourier::ExponentialMovingDecomposition,
        smoothing::{Exponential, HalfLife},
    },
    ops::interval::Interval,
    prelude::*,
    quantity::{
        Zero,
        angle::{Harmonic, Radians},
        energy::{DecawattHours, MilliwattHours, WattHours},
        power::Watts,
        time::Hours,
    },
};

#[must_use]
#[derive(Clone, Encode, Decode)]
pub struct Profile {
    /// Battery profile.
    #[musli(Binary, name = 13)]
    #[musli(default)]
    pub battery: Battery,

    #[musli(Binary, name = 14)]
    #[musli(default)]
    pub energy: Energy,
}

impl Profile {
    const PATH: &str = "energy-profile.musli";

    #[instrument]
    pub async fn read_from_file(n_balance_harmonics: usize) -> Result<Self> {
        let path = Path::new(Self::PATH);
        Ok(if path.exists() {
            let bytes = tokio::fs::read(path).await.context("failed to read the file")?;
            let mut this: Self =
                wire::decode(bytes.as_slice()).context("failed to decode the file")?;
            this.energy.balance.resize(n_balance_harmonics);
            this
        } else {
            Self { battery: Battery::default(), energy: Energy::new(n_balance_harmonics) }
        })
    }

    #[instrument(skip_all, fields(path = Self::PATH))]
    pub async fn write_to_file(&self) -> Result {
        let final_path = Path::new(Self::PATH);
        let temporary_path = final_path.with_added_extension("temporary");

        let bytes = wire::to_vec(self).context("failed to encode the energy profile")?;
        tokio::fs::write(&temporary_path, bytes.as_slice())
            .await
            .context("failed to write the energy profile")?;
        tokio::fs::rename(&temporary_path, final_path)
            .await
            .context("failed to rename the temporary file")?;
        Ok(())
    }
}

#[derive(Copy, Clone, Encode, Decode)]
pub struct Battery {
    #[musli(Binary, name = 1)]
    pub efficiency: energy::Flow<f64>,

    #[musli(Binary, name = 2)]
    pub tracker: Option<BatteryTracker>,
}

impl Default for Battery {
    fn default() -> Self {
        Self { efficiency: energy::Flow { import: 0.95, export: 0.95 }, tracker: None }
    }
}

impl Battery {
    /// Track the battery metrics and update the battery efficiency parameters when the residual energy has changed.
    ///
    /// # Returns
    ///
    /// - [`true`], if the battery residual energy has changed since the last call;
    /// - [`false`], otherwise.
    #[instrument(skip_all)]
    #[must_use]
    pub fn track(&mut self, current_metrics: &mini_qube::Metrics, half_life_factor: f64) -> bool {
        let current_tracker = BatteryTracker {
            total_grid_flow: current_metrics.total_grid_flow,
            residual_energy: current_metrics.residual_energy(),
        };
        let Some(tracker) = &self.tracker else {
            self.tracker = Some(current_tracker);
            info!("initialized battery tracker");
            return true;
        };

        let residual_energy_change = current_metrics.residual_energy() - tracker.residual_energy;
        if residual_energy_change == Zero::ZERO {
            // Keep accumulating the grid flow until the residual energy changes.
            return false;
        }

        let residual_energy_change = WattHours::from(residual_energy_change).abs();
        let grid_flow = current_metrics.total_grid_flow - tracker.total_grid_flow;

        match (grid_flow.import == Zero::ZERO, grid_flow.export == Zero::ZERO) {
            (true, false) => {
                let grid_export = grid_flow.export.rescale();
                let efficiency = grid_export / residual_energy_change;
                let smoothing_factor =
                    HalfLife(current_metrics.actual_capacity() * half_life_factor)
                        .smoothing_factor(grid_export);
                self.efficiency.export =
                    Exponential(self.efficiency.export).update(efficiency, smoothing_factor).0;
                info!(
                    ?residual_energy_change,
                    ?grid_export,
                    ?efficiency,
                    ?smoothing_factor,
                    "discharging",
                );
            }
            (false, true) => {
                let grid_import = grid_flow.import.rescale();
                let efficiency = residual_energy_change / grid_import;
                let smoothing_factor =
                    HalfLife(current_metrics.actual_capacity() * half_life_factor)
                        .smoothing_factor(grid_import);
                self.efficiency.import =
                    Exponential(self.efficiency.import).update(efficiency, smoothing_factor).0;
                info!(
                    ?residual_energy_change,
                    ?grid_import,
                    ?efficiency,
                    ?smoothing_factor,
                    "charging",
                );
            }
            (false, false) | (true, true) => {
                info!("ignoring idle or mixed regime");
            }
        }

        self.tracker = Some(current_tracker);
        true
    }
}

#[derive(Copy, Clone, Encode, Decode)]
pub struct BatteryTracker {
    #[musli(Binary, name = 1)]
    pub total_grid_flow: energy::Flow<DecawattHours>,

    #[musli(Binary, name = 2)]
    pub residual_energy: MilliwattHours,
}

#[derive(Clone, Encode, Decode)]
pub struct Energy {
    /// Timestamp of the last update to the parameters.
    #[musli(Binary, name = 1)]
    #[musli(with = crate::ops::musli::chrono)]
    updated_at: DateTime<Local>,

    /// Average EPS active power.
    #[musli(Binary, name = 2)]
    pub eps_active_power: Exponential<Watts>,

    /// Global average energy balance (constant term of the Fourier decomposition).
    #[deprecated]
    #[musli(Binary, name = 3)]
    mean: Exponential<energy::Balance<Watts>>,

    /// Energy balance harmonics (c₁ and so on).
    #[deprecated]
    #[musli(Binary, name = 4)]
    harmonics: Vec<Exponential<Harmonic<energy::Balance<Watts>>>>,

    /// TODO: drop `default` after migration.
    #[musli(Binary, name = 5)]
    #[musli(default)]
    pub balance: ExponentialMovingDecomposition<energy::Balance<Watts>>,
}

impl Default for Energy {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Energy {
    const DEFAULT_HARMONIC: Exponential<Harmonic<energy::Balance<Watts>>> = Exponential(Zero::ZERO);

    fn new(n_balance_harmonics: usize) -> Self {
        Self {
            updated_at: Local::now(),
            eps_active_power: Exponential(Watts::ZERO),

            #[expect(deprecated)]
            mean: Exponential(Zero::ZERO),

            #[expect(deprecated)]
            harmonics: vec![Self::DEFAULT_HARMONIC; n_balance_harmonics],

            balance: ExponentialMovingDecomposition::new(n_balance_harmonics),
        }
    }

    /// Calculate the balance deviation from the average at the given moment in time.
    pub fn deviation_at(&self, naive_time: NaiveTime) -> energy::Balance<Watts> {
        self.balance.deviation_at(Radians::daily_phase_at(naive_time))
    }

    pub fn normalized_mean_over(
        &self,
        interval: Interval<DateTime<Local>>,
    ) -> energy::Balance<Watts> {
        let mean_deviation = {
            let start_phase = Radians::daily_phase_at(interval.start().time());
            // FIXME: strictly speaking this is not correct for DST transitions:
            let end_phase = start_phase + Radians::daily_phase_shift_of(interval.duration());
            self.balance.mean_deviation_over(start_phase..end_phase)
        };
        let balance = self.balance.mean() + mean_deviation;
        energy::Balance { grid: balance.grid.normalized(), battery: balance.battery.normalized() }
    }

    #[instrument(skip_all)]
    pub fn update(
        &mut self,
        balance: energy::Balance<Watts>,
        eps_active_power: Watts,
        at: DateTime<Local>,
        half_life: HalfLife<Hours>,
    ) {
        let mean_smoothing_factor = {
            // Smoothing factor based on the configured half-life and elapsed time:
            let elapsed = at - std::mem::replace(&mut self.updated_at, at);
            half_life.smoothing_factor(elapsed)
        };

        self.eps_active_power.update(eps_active_power, mean_smoothing_factor);
        self.balance.update(balance, Radians::daily_phase_at(at.time()), mean_smoothing_factor);
    }
}
