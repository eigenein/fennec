use std::{
    f64::consts::{PI, TAU},
    path::Path,
};

use chrono::{DateTime, Local, NaiveTime, Timelike};
use musli::{Decode, Encode, wire};

use crate::{
    api::mini_qube,
    energy,
    math::smoothing::{Exponential, HalfLife},
    ops::interval::Interval,
    prelude::*,
    quantity::{
        Quantity,
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
    pub balance: Balance,
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
            this.balance.resize(n_balance_harmonics);
            this
        } else {
            Self { battery: Battery::default(), balance: Balance::new(n_balance_harmonics) }
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
        info!(?residual_energy_change, "changed");
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
                debug!("idle and mixed regimes are ignored");
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
pub struct Balance {
    /// Timestamp of the last update to the parameters.
    #[musli(Binary, name = 1)]
    #[musli(with = crate::ops::musli::chrono)]
    updated_at: DateTime<Local>,

    /// Average EPS active power.
    #[musli(Binary, name = 2)]
    pub eps_active_power: Exponential<Watts>,

    /// Global average energy balance (constant term of the Fourier decomposition).
    #[musli(Binary, name = 3)]
    pub mean: Exponential<energy::Balance<Watts>>,

    /// Energy balance harmonics (c₁ and so on).
    #[musli(Binary, name = 4)]
    pub harmonics: Vec<Exponential<Harmonic<energy::Balance<Watts>>>>,
}

impl Default for Balance {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Balance {
    const DEFAULT_HARMONIC: Exponential<Harmonic<energy::Balance<Watts>>> = Exponential(Zero::ZERO);

    fn new(n_balance_harmonics: usize) -> Self {
        Self {
            updated_at: Local::now(),
            eps_active_power: Exponential(Watts::ZERO),
            mean: Exponential(Zero::ZERO),
            harmonics: vec![Self::DEFAULT_HARMONIC; n_balance_harmonics],
        }
    }

    fn resize(&mut self, n_balance_harmonics: usize) {
        self.harmonics.resize(n_balance_harmonics, Self::DEFAULT_HARMONIC);
    }

    /// Calculate the balance deviation from the average at concrete moment in time.
    pub fn deviation_at(&self, naive_time: NaiveTime) -> energy::Balance<Watts> {
        let day_phase: Radians =
            Quantity(f64::from(naive_time.num_seconds_from_midnight()) / 86400.0 * TAU);
        (1..)
            .map(f64::from)
            .zip(self.harmonics.iter())
            .map(|(mode_index, harmonic)| {
                harmonic.0.dot(Harmonic::from_phase(day_phase * mode_index))
            })
            .fold(energy::Balance::ZERO, |sum, item| sum + item)
    }

    pub fn mean_over(&self, interval: Interval<DateTime<Local>>) -> energy::Balance<Watts> {
        /// Noise threshold for the resulting balance.
        ///
        /// The Fourier decomposition produces small oscillations like tiny PV charge at night.
        /// That caused certain schedule slots to become unstable under re-optimization
        /// (for example, frequent switching between "self-use" and "feed-in priority" modes).
        const NOISE_THRESHOLD: Watts = Watts::TEN;

        let balance = self.mean.0 + self.mean_deviation_over(interval);
        energy::Balance {
            grid: balance.grid.normalized().denoised(NOISE_THRESHOLD),
            battery: balance.battery.normalized().denoised(NOISE_THRESHOLD),
        }
    }

    /// Calculate the mean deviation of the balance over the interval.
    fn mean_deviation_over(&self, interval: Interval<DateTime<Local>>) -> energy::Balance<Watts> {
        assert!(interval.start() < interval.end());

        let n_days = Hours::from(interval.duration()).days();
        let middle_phase: Radians = {
            let start =
                f64::from(interval.start().time().num_seconds_from_midnight()) / 86400.0 * TAU;
            Quantity((n_days / 2.0).mul_add(TAU, start))
        };

        (1..)
            .map(f64::from)
            .zip(self.harmonics.iter())
            .map(|(mode_index, harmonic)| {
                // (1/Δt) ∫ cos(2πk·t) dt = cos(2πk·middle_phase) · sinc(k·Δt)
                let weight = Self::sinc(mode_index * n_days);
                harmonic.0.dot(Harmonic::from_phase(middle_phase * mode_index)) * weight
            })
            .fold(energy::Balance::ZERO, |sum, item| sum + item)
    }

    /// Normalized [sinc function](https://en.wikipedia.org/wiki/Sinc_function): sin(πx)÷(πx).
    fn sinc(x: f64) -> f64 {
        if x == 0.0 {
            1.0
        } else {
            let pi_x = PI * x;
            pi_x.sin() / pi_x
        }
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

        // Calculate the deviation before the mean update eats the signal:
        let deviation = balance - self.mean.0;

        self.mean.update(balance, mean_smoothing_factor);

        // Capture daily periodicity, hence one full day is τ radians:
        let base_phase: Radians =
            Quantity(f64::from(at.time().num_seconds_from_midnight()) / 86400.0 * TAU);

        // After long gaps, the smoothing factor jumps through the roof, and
        // each harmonic would then pick up the full signal – effectively amplifying it by N.
        // The following ensures that α × 2N ≤ 1 and the spike is constrained:
        #[expect(clippy::cast_precision_loss)]
        let harmonic_smoothing_factor =
            mean_smoothing_factor.min(0.5 / self.harmonics.len() as f64);

        for (mode_index, harmonic) in (1..).map(f64::from).zip(self.harmonics.iter_mut()) {
            let basis = Harmonic::from_phase(base_phase * mode_index);
            let target = Harmonic {
                // Multiplication by 2 comes from the scale factor:
                // <https://en.wikipedia.org/wiki/Fourier_series#Analysis>.
                cosine: deviation * (2.0 * basis.cosine),
                sine: deviation * (2.0 * basis.sine),
            };
            harmonic.update(target, harmonic_smoothing_factor);
        }
    }
}
