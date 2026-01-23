use std::time::{Duration, Instant};

use bon::Builder;
use futures_core::TryStream;
use futures_util::TryStreamExt;
use linfa::{
    Dataset,
    dataset::Records,
    traits::{Fit, Predict},
};
use linfa_linear::LinearRegression;
use ndarray::{Array1, Array2, Axis, aview0, aview1};
use tokio::pin;

use crate::{
    core::interval::Interval,
    db::{
        Db,
        measurements::{Measurement, Measurements},
    },
    fmt::FormattedEfficiency,
    prelude::*,
    quantity::{Quantity, power::Kilowatts},
};

#[must_use]
#[derive(Copy, Clone, Builder)]
pub struct BatteryEfficiency {
    pub parasitic_load: Kilowatts,

    /// Charging efficiency, `0..=1`.
    pub charging: f64,

    /// Discharging efficiency, `0..=1`.
    pub discharging: f64,
}

impl BatteryEfficiency {
    pub const fn round_trip(&self) -> f64 {
        self.charging * self.discharging
    }

    pub async fn try_estimate_from(db: &Db, duration: Duration) -> Result<Self> {
        let measurements = Measurements(db);
        let stream = measurements
            .select(Interval::try_since(duration)?)
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
    pub async fn try_estimate<S>(mut measurements: S) -> Result<Self>
    where
        S: TryStream<Ok = Measurement, Error = Error> + Unpin,
    {
        let mut previous_measurement =
            measurements.try_next().await?.context("empty measurement stream")?;
        let mut dataset =
            Dataset::new(Array2::zeros((0, 3)), Array1::zeros(0)).with_weights(Array1::zeros(0));

        info!("reading the measurements…");
        while let Some(measurement) = measurements.try_next().await? {
            let imported_energy = measurement.battery.import - previous_measurement.battery.import;
            let exported_energy = measurement.battery.export - previous_measurement.battery.export;
            let residual_differential =
                measurement.residual_energy - previous_measurement.residual_energy;
            let duration = measurement.timestamp - previous_measurement.timestamp;
            let weight = {
                let energy_signal = imported_energy + exported_energy;
                let parasitic_signal = Kilowatts::from(0.02) * duration;
                let weight = energy_signal + parasitic_signal;

                #[expect(clippy::cast_possible_truncation)]
                let weight = weight.0 as f32;

                weight
            };

            dataset.records.push_row(aview1(&[
                imported_energy.0,
                exported_energy.0,
                duration.as_seconds_f64() / 3600.0,
            ]))?;
            dataset.targets.push(Axis(0), aview0(&residual_differential.0))?;
            dataset.weights.push(Axis(0), aview0(&weight))?;

            previous_measurement = measurement;
        }

        info!(n_records = dataset.nsamples(), "estimating the battery efficiency…");
        let start_time = Instant::now();
        let regression = LinearRegression::new().with_intercept(false).fit(&dataset)?;

        info!(elapsed = ?start_time.elapsed(), "evaluating…");
        let r_squared = {
            let predictions = regression.predict(&dataset.records);
            let residual_sum_of_squares =
                (&dataset.targets - predictions).mapv(|value| value.powi(2)).sum();
            let mean = dataset.targets.mean().unwrap();
            let total_sum_of_squares = dataset.targets.mapv(|value| (value - mean).powi(2)).sum();
            1.0 - residual_sum_of_squares / total_sum_of_squares
        };
        info!(r_squared, "evaluated");

        Ok(Self::builder()
            .charging(regression.params()[0])
            .discharging(-1.0 / regression.params()[1])
            .parasitic_load(Quantity(-regression.params()[2]))
            .build())
    }
}
