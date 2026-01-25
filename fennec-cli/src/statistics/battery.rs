use std::time::{Duration, Instant};

use bon::bon;
use futures_core::TryStream;
use futures_util::TryStreamExt;
use linfa::{Dataset, dataset::Records, traits::Fit};
use linfa_linear::LinearRegression;
use ndarray::{Array1, Array2, Axis, aview0, aview1};
use tokio::pin;

use crate::{
    core::interval::Interval,
    db::{Db, battery_log::BatteryLog},
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

    pub async fn try_estimate_from(db: &Db, duration: Duration) -> Result<Self> {
        let stream = BatteryLog::select_from(db, Interval::try_since(duration)?)
            .await
            .context("failed to query the measurements")?;
        pin!(stream);
        let efficiency = Self::try_estimate(stream)
            .await
            .context("failed to estimate the battery efficiency")?;
        info!(
            parasitic_load = ?efficiency.parasitic_load,
            round_trip = ?FormattedEfficiency(efficiency.round_trip()),
            charging = ?FormattedEfficiency(efficiency.charging),
            discharging = ?FormattedEfficiency(efficiency.discharging),
            "completed",
        );
        Ok(efficiency)
    }

    #[instrument(skip_all)]
    pub async fn try_estimate<S>(mut battery_logs: S) -> Result<Self>
    where
        S: TryStream<Ok = BatteryLog, Error = Error> + Unpin,
    {
        let mut previous_measurement =
            battery_logs.try_next().await?.context("empty battery log stream")?;
        let mut dataset =
            Dataset::new(Array2::zeros((0, 3)), Array1::zeros(0)).with_weights(Array1::zeros(0));

        info!("reading the battery logs…");
        while let Some(log) = battery_logs.try_next().await? {
            let imported_energy = log.meter.import - previous_measurement.meter.import;
            let exported_energy = log.meter.export - previous_measurement.meter.export;
            let residual_differential = log.residual_energy - previous_measurement.residual_energy;
            let duration = log.timestamp - previous_measurement.timestamp;

            dataset.records.push_row(aview1(&[
                imported_energy.0,
                exported_energy.0,
                duration.as_seconds_f64() / 3600.0,
            ]))?;
            dataset.targets.push(Axis(0), aview0(&residual_differential.0))?;
            dataset.weights.push(Axis(0), aview0(&duration.as_seconds_f32()))?;

            previous_measurement = log;
        }
        if dataset.nsamples() == 0 {
            bail!("empty dataset, collect samples first");
        }

        info!(n_records = dataset.nsamples(), "estimating the battery efficiency…");
        let start_time = Instant::now();
        let regression = LinearRegression::new()
            .with_intercept(false)
            .fit(&dataset)
            .context("failed to fit a regression, try with again with more samples")?;
        info!(elapsed = ?start_time.elapsed(), "regression has been fit");

        Self::builder()
            .charging(regression.params()[0])
            .discharging(-1.0 / regression.params()[1])
            .parasitic_load(Quantity(-regression.params()[2]))
            .build()
    }
}
