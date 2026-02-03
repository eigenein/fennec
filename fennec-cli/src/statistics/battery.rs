use std::time::Instant;

use bon::bon;
use futures_core::TryStream;
use futures_util::TryStreamExt;
use linfa::{
    Dataset,
    dataset::Records,
    traits::{Fit, Predict},
};
use linfa_linear::{FittedLinearRegression, LinearRegression};
use ndarray::{Array1, Array2, Axis, Ix1, aview0, aview1};

use crate::{
    db::battery::BatteryLog,
    fmt::FormattedEfficiency,
    prelude::*,
    quantity::{Quantity, power::Kilowatts},
};

#[must_use]
#[derive(Copy, Clone)]
pub struct BatteryEfficiency {
    pub parasitic_load: Kilowatts,

    /// Charging efficiency, `0..=1`.
    pub charging: f64,

    /// Discharging efficiency, `0..=1`.
    pub discharging: f64,
}

#[bon]
impl BatteryEfficiency {
    #[builder]
    pub fn new(parasitic_load: Kilowatts, charging: f64, discharging: f64) -> Result<Self> {
        if parasitic_load.0.is_nan()
            || parasitic_load.0.is_infinite()
            || parasitic_load < Kilowatts::ZERO
        {
            bail!("invalid parasitic load: {parasitic_load}");
        }
        if charging.is_nan() || charging.is_infinite() {
            bail!("invalid charging efficiency: {charging}");
        }
        if discharging.is_nan() || discharging.is_infinite() {
            bail!("invalid discharging efficiency: {discharging}");
        }
        Ok(Self { parasitic_load, charging, discharging })
    }
}

impl BatteryEfficiency {
    pub const fn round_trip(&self) -> f64 {
        self.charging * self.discharging
    }

    #[instrument(skip_all)]
    pub async fn try_estimate<S>(mut battery_logs: S) -> Result<Self>
    where
        S: TryStream<Ok = BatteryLog, Error = Error> + Unpin,
    {
        let mut previous = battery_logs.try_next().await?.context("empty battery log stream")?;
        let mut dataset = Dataset::new(Array2::zeros((0, 3)), Array1::zeros(0));

        info!("reading the battery logs…");
        while let Some(log) = battery_logs.try_next().await? {
            let imported_energy = log.metrics.import - previous.metrics.import;
            let exported_energy = log.metrics.export - previous.metrics.export;
            let duration = log.timestamp - previous.timestamp;
            let residual_differential = log.residual_energy - previous.residual_energy;

            dataset.records.push_row(aview1(&[
                imported_energy.0,
                exported_energy.0,
                duration.as_seconds_f64() / 3600.0,
            ]))?;
            dataset.targets.push(Axis(0), aview0(&residual_differential.0))?;

            previous = log;
        }
        if dataset.nsamples() == 0 {
            bail!("empty dataset");
        }

        info!(n_records = dataset.nsamples(), "estimating the battery efficiency…");
        let start_time = Instant::now();
        let regression = LinearRegression::new()
            .with_intercept(false)
            .fit(&dataset)
            .context("failed to fit a regression, try with again with more samples")?;
        info!(elapsed = ?start_time.elapsed(), "regression has been fit");

        let r_squared = Self::r_squared(&regression, &dataset);
        info!(r_squared, "evaluated");

        let this = Self::builder()
            .charging(regression.params()[0])
            .discharging(-1.0 / regression.params()[1])
            .parasitic_load(Quantity(-regression.params()[2]))
            .build()?;
        info!(
            parasitic_load = ?this.parasitic_load,
            round_trip = ?FormattedEfficiency(this.round_trip()),
            charging = ?FormattedEfficiency(this.charging),
            discharging = ?FormattedEfficiency(this.discharging),
            "completed",
        );
        Ok(this)
    }

    fn r_squared(
        regression: &FittedLinearRegression<f64>,
        dataset: &Dataset<f64, f64, Ix1>,
    ) -> f64 {
        let predicted = regression.predict(dataset);
        let target_mean = dataset.targets().mean().unwrap();
        let residual_squared_sum = (dataset.targets() - &predicted).mapv(|diff| diff * diff).sum();
        let total_squares_sum = (dataset.targets() - target_mean).mapv(|diff| diff * diff).sum();
        1.0 - residual_squared_sum / total_squares_sum
    }
}
