use std::{
    f64::consts::{PI, TAU},
    path::Path,
};

use chrono::{DateTime, Local, NaiveTime, Timelike};
use musli::{Decode, Encode, wire};

use super::{Balance, Flow};
use crate::{
    api::mini_qube,
    math::smoothing::{Exponential, HalfLife},
    ops::chrono::Interval,
    prelude::*,
    quantity::{
        Quantity,
        Zero,
        angle::{Harmonic, Radians},
        energy::WattHours,
        power::Watts,
        time::Hours,
    },
};

#[must_use]
#[derive(Encode, Decode)]
pub struct Profile {
    /// Timestamp of the last update to the parameters.
    ///
    /// It does not apply to the battery metrics and parameters.
    /// It would be nice to use something like `#[musli(flatten)]` and extract the structure,
    /// but Musli does not support this at the moment.
    #[musli(Binary, name = 6)]
    #[musli(with = crate::ops::musli::chrono)]
    balance_updated_at: DateTime<Local>,

    /// Average EPS active power.
    #[musli(Binary, name = 7)]
    pub eps_active_power: Exponential<Watts>,

    /// Global average energy balance (constant term of the Fourier decomposition).
    #[musli(Binary, name = 8)]
    pub mean_balance: Exponential<Balance<Watts>>,

    /// Energy balance harmonics (c₁ and so on).
    #[musli(Binary, name = 9)]
    pub balance_harmonics: Vec<Exponential<Harmonic<Balance<Watts>>>>,

    /// Battery metrics as read from the device.
    ///
    /// This attribute is updated if and only if the residual charge changes.
    /// Hence, it has its own timestamp.
    #[musli(Binary, name = 10)]
    #[musli(default)]
    pub battery_metrics: Option<mini_qube::TrackedMetrics>,

    #[musli(Binary, name = 12)]
    #[musli(default = Self::default_battery_efficiency)]
    pub battery_efficiency: Flow<f64>,
}

impl Profile {
    const PATH: &str = "energy-profile.musli";
    const DEFAULT_HARMONIC: Exponential<Harmonic<Balance<Watts>>> = Exponential(Harmonic::ZERO);

    #[instrument]
    pub async fn read_from_file(n_balance_harmonics: usize) -> Result<Self> {
        let path = Path::new(Self::PATH);
        Ok(if path.exists() {
            let bytes = tokio::fs::read(path).await.context("failed to read the file")?;
            let mut this: Self =
                wire::decode(bytes.as_slice()).context("failed to decode the file")?;
            this.balance_harmonics.resize(n_balance_harmonics, Self::DEFAULT_HARMONIC);
            this
        } else {
            Self {
                balance_updated_at: Local::now(),
                mean_balance: Exponential(Balance::ZERO),
                eps_active_power: Exponential(Watts::ZERO),
                balance_harmonics: vec![Self::DEFAULT_HARMONIC; n_balance_harmonics],
                battery_metrics: None,
                battery_efficiency: Self::default_battery_efficiency(),
            }
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

    /// Track the battery metrics and update the battery efficiency parameters when the residual energy has changed.
    ///
    /// # Returns
    ///
    /// - [`true`], if the battery residual energy has changed since the last call;
    /// - [`false`], otherwise.
    #[instrument(skip_all)]
    #[must_use]
    pub fn track_battery_metrics(
        &mut self,
        current_metrics: mini_qube::TrackedMetrics,
        half_life_factor: f64,
    ) -> bool {
        let Some(last_metrics) = &self.battery_metrics else {
            // First initialization:
            self.battery_metrics = Some(current_metrics);
            return true;
        };

        let residual_energy_change =
            current_metrics.residual_energy() - last_metrics.residual_energy();
        if residual_energy_change == Zero::ZERO {
            // No change in the residual energy: do not update the parameters and keep accumulating.
            return false;
        }

        let residual_energy_change = WattHours::from(residual_energy_change).abs();
        let grid_flow = current_metrics.total_grid_flow - last_metrics.total_grid_flow;

        match (grid_flow.import == Zero::ZERO, grid_flow.export == Zero::ZERO) {
            (true, false) => {
                let grid_export = grid_flow.export.rescale();
                let efficiency = grid_export / residual_energy_change;
                let smoothing_factor =
                    HalfLife(current_metrics.actual_capacity() * half_life_factor)
                        .smoothing_factor(grid_export);
                self.battery_efficiency.export = Exponential(self.battery_efficiency.export)
                    .update(efficiency, smoothing_factor)
                    .0;
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
                self.battery_efficiency.import = Exponential(self.battery_efficiency.import)
                    .update(efficiency, smoothing_factor)
                    .0;
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

        self.battery_metrics = Some(current_metrics);
        true
    }

    #[instrument(skip_all)]
    pub fn update_energy_balance(
        &mut self,
        balance: Balance<Watts>,
        eps_active_power: Watts,
        at: DateTime<Local>,
        half_life: HalfLife<Hours>,
    ) {
        let mean_smoothing_factor = {
            // Smoothing factor based on the configured half-life and elapsed time:
            let elapsed = at - std::mem::replace(&mut self.balance_updated_at, at);
            half_life.smoothing_factor(elapsed)
        };

        self.eps_active_power.update(eps_active_power, mean_smoothing_factor);

        // Calculate the deviation before the mean update eats the signal:
        let deviation = balance - self.mean_balance.0;

        self.mean_balance.update(balance, mean_smoothing_factor);

        // Capture daily periodicity, hence one full day is τ radians:
        let base_phase: Radians =
            Quantity(f64::from(at.time().num_seconds_from_midnight()) / 86400.0 * TAU);

        // After long gaps, the smoothing factor jumps through the roof, and
        // each harmonic would then pick up the full signal – effectively amplifying it by N.
        // The following ensures that α × 2N ≤ 1 and the spike is constrained:
        #[expect(clippy::cast_precision_loss)]
        let harmonic_smoothing_factor =
            mean_smoothing_factor.min(0.5 / self.balance_harmonics.len() as f64);

        for (mode_index, harmonic) in (1..).map(f64::from).zip(self.balance_harmonics.iter_mut()) {
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

    /// Calculate the balance deviation from the average at concrete moment in time.
    pub fn deviation_at(&self, naive_time: NaiveTime) -> Balance<Watts> {
        let day_phase: Radians =
            Quantity(f64::from(naive_time.num_seconds_from_midnight()) / 86400.0 * TAU);
        (1..)
            .map(f64::from)
            .zip(self.balance_harmonics.iter())
            .map(|(mode_index, harmonic)| {
                harmonic.0.dot(Harmonic::from_phase(day_phase * mode_index))
            })
            .fold(Balance::ZERO, |sum, item| sum + item)
    }

    pub fn mean_balance_over(&self, interval: Interval) -> Balance<Watts> {
        let balance = self.mean_balance.0 + self.mean_deviation_over(interval);
        Balance { grid: balance.grid.normalized(), battery: balance.battery.normalized() }
    }

    /// Calculate the mean deviation of the balance over the interval.
    fn mean_deviation_over(&self, interval: Interval) -> Balance<Watts> {
        assert!(interval.start() < interval.end());

        let n_days = interval.duration().days();
        let middle_phase: Radians = {
            let start =
                f64::from(interval.start().time().num_seconds_from_midnight()) / 86400.0 * TAU;
            Quantity((n_days / 2.0).mul_add(TAU, start))
        };

        (1..)
            .map(f64::from)
            .zip(self.balance_harmonics.iter())
            .map(|(mode_index, harmonic)| {
                // (1/Δt) ∫ cos(2πk·t) dt = cos(2πk·middle_phase) · sinc(k·Δt)
                let weight = sinc(mode_index * n_days);
                harmonic.0.dot(Harmonic::from_phase(middle_phase * mode_index)) * weight
            })
            .fold(Balance::ZERO, |sum, item| sum + item)
    }

    const fn default_battery_efficiency() -> Flow<f64> {
        Flow { import: 0.95, export: 0.95 }
    }
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
