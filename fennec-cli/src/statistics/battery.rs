use std::{
    fmt::{Display, Formatter},
    time::Instant,
};

use bon::bon;
use comfy_table::{Attribute, Cell, CellAlignment, Color, Table, modifiers, presets};
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
    db::battery::Measurement,
    fmt::FormattedPercentage,
    prelude::*,
    quantity::{Zero, energy::WattHours, power::Watts, time::Hours},
};

#[must_use]
#[derive(Copy, Clone)]
pub struct BatteryEfficiency {
    pub parasitic_load: Watts,

    /// Charging efficiency, `0..=1`.
    pub charging: f64,

    /// Discharging efficiency, `0..=1`.
    pub discharging: f64,

    pub n_samples: usize,

    pub total_hours: Hours,
}

impl Default for BatteryEfficiency {
    fn default() -> Self {
        Self {
            parasitic_load: Watts::ZERO,
            charging: 1.0,
            discharging: 1.0,
            n_samples: 0,
            total_hours: Hours::ZERO,
        }
    }
}

#[bon]
impl BatteryEfficiency {
    #[builder]
    pub fn new(
        parasitic_load: Watts,
        charging: f64,
        discharging: f64,
        n_samples: usize,
        total_hours: Hours,
    ) -> Result<Self> {
        if parasitic_load.0.is_nan()
            || parasitic_load.0.is_infinite()
            || parasitic_load < Watts::ZERO
        {
            bail!("invalid parasitic load: {parasitic_load}");
        }
        if charging.is_nan() || charging.is_infinite() {
            bail!("invalid charging efficiency: {charging}");
        }
        if discharging.is_nan() || discharging.is_infinite() {
            bail!("invalid discharging efficiency: {discharging}");
        }
        Ok(Self { parasitic_load, charging, discharging, n_samples, total_hours })
    }
}

impl BatteryEfficiency {
    pub const fn round_trip(&self) -> f64 {
        self.charging * self.discharging
    }

    #[instrument(skip_all)]
    pub async fn try_estimate<S>(mut battery_logs: S) -> Result<Self>
    where
        S: TryStream<Ok = Measurement, Error = Error> + Unpin,
    {
        let mut previous = battery_logs.try_next().await?.context("empty battery log stream")?;
        let mut dataset = Dataset::new(Array2::zeros((0, 3)), Array1::zeros(0));
        let mut total_time = Hours::ZERO;

        info!("reading the battery logs…");
        while let Some(log) = battery_logs.try_next().await? {
            let imported_energy = WattHours::from(log.legacy_import - previous.legacy_import);
            let exported_energy = WattHours::from(log.legacy_export - previous.legacy_export);
            let time_delta = Hours::from(log.timestamp - previous.timestamp);
            total_time += time_delta;
            let residual_differential =
                WattHours::from(log.legacy_residual_energy - previous.legacy_residual_energy);

            dataset.records.push_row(aview1(&[
                imported_energy.0,
                exported_energy.0,
                time_delta.0,
            ]))?;
            dataset.targets.push(Axis(0), aview0(&(residual_differential.0)))?;

            previous = log;
        }
        if dataset.nsamples() == 0 {
            bail!("empty dataset");
        }

        info!(n_samples = dataset.nsamples(), "estimating the battery efficiency…");
        let start_time = Instant::now();
        let regression = LinearRegression::new()
            .with_intercept(false)
            .fit(&dataset)
            .context("failed to fit a regression")?;
        info!(elapsed = ?start_time.elapsed(), "completed");

        let this = Self::builder()
            .charging(regression.params()[0])
            .discharging(-1.0 / regression.params()[1])
            .parasitic_load(Watts(-regression.params()[2]))
            .n_samples(dataset.nsamples())
            .total_hours(total_time)
            .build()?;
        println!("{this}");
        Ok(this)
    }

    #[expect(unused)]
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

impl Display for BatteryEfficiency {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec![Cell::from("Battery")])
            .add_row(vec![Cell::from("Total time"), Cell::from(self.total_hours)])
            .add_row(vec![
                Cell::from("Samples"),
                Cell::from(self.n_samples).set_alignment(CellAlignment::Right),
            ])
            .add_row(vec![
                Cell::from("Charging").fg(Color::Green),
                Cell::from(FormattedPercentage(self.charging))
                    .set_alignment(CellAlignment::Right)
                    .fg(Color::Green)
                    .add_attribute(Attribute::Bold),
            ])
            .add_row(vec![
                Cell::from("Discharging").fg(Color::Red),
                Cell::from(FormattedPercentage(self.discharging))
                    .set_alignment(CellAlignment::Right)
                    .fg(Color::Red)
                    .add_attribute(Attribute::Bold),
            ])
            .add_row(vec![
                Cell::from("Parasitic load"),
                Cell::from(self.parasitic_load)
                    .add_attribute(Attribute::Bold)
                    .set_alignment(CellAlignment::Right),
            ])
            .add_row(vec![
                Cell::from("Round trip").fg(Color::DarkYellow),
                Cell::from(FormattedPercentage(self.round_trip()))
                    .set_alignment(CellAlignment::Right)
                    .fg(Color::DarkYellow)
                    .add_attribute(Attribute::Bold),
            ]);
        write!(f, "{table}")
    }
}
