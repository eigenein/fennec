use std::{
    fmt::{Display, Formatter},
    time::Instant,
};

use bon::bon;
use chrono::TimeDelta;
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
    cli::WeightMode,
    db::battery::BatteryLog,
    fmt::FormattedPercentage,
    prelude::*,
    quantity::{Quantity, energy::KilowattHours, power::Kilowatts},
};

#[must_use]
#[derive(Copy, Clone)]
pub struct BatteryEfficiency {
    pub parasitic_load: Kilowatts,

    /// Charging efficiency, `0..=1`.
    pub charging: f64,

    /// Discharging efficiency, `0..=1`.
    pub discharging: f64,

    pub n_samples: usize,

    pub total_time: TimeDelta,
}

impl Default for BatteryEfficiency {
    fn default() -> Self {
        Self {
            parasitic_load: Kilowatts::ZERO,
            charging: 1.0,
            discharging: 1.0,
            n_samples: 0,
            total_time: TimeDelta::zero(),
        }
    }
}

#[bon]
impl BatteryEfficiency {
    #[builder]
    pub fn new(
        parasitic_load: Kilowatts,
        charging: f64,
        discharging: f64,
        n_samples: usize,
        total_time: TimeDelta,
    ) -> Result<Self> {
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
        Ok(Self { parasitic_load, charging, discharging, n_samples, total_time })
    }
}

impl BatteryEfficiency {
    pub const fn round_trip(&self) -> f64 {
        self.charging * self.discharging
    }

    #[instrument(skip_all)]
    pub async fn try_estimate<S>(mut battery_logs: S, weight_mode: WeightMode) -> Result<Self>
    where
        S: TryStream<Ok = BatteryLog, Error = Error> + Unpin,
    {
        let mut previous = battery_logs.try_next().await?.context("empty battery log stream")?;
        let mut dataset = Dataset::new(Array2::zeros((0, 3)), Array1::zeros(0));
        let mut total_time = TimeDelta::zero();

        info!("reading the battery logs…");
        while let Some(log) = battery_logs.try_next().await? {
            let imported_energy = log.metrics.import - previous.metrics.import;
            let exported_energy = log.metrics.export - previous.metrics.export;
            let hours = {
                let time_delta = log.timestamp - previous.timestamp;
                total_time += time_delta;
                time_delta.as_seconds_f64() / 3600.0
            };
            let residual_differential = log.residual_energy - previous.residual_energy;

            let weight_multiplier = {
                let weight = match weight_mode {
                    WeightMode::None => 1.0,
                    WeightMode::EnergyFlow => {
                        (imported_energy + exported_energy + KilowattHours::ONE_WATT_HOUR).0
                    }
                };
                weight.sqrt()
            };

            dataset.records.push_row(aview1(&[
                imported_energy.0 * weight_multiplier,
                exported_energy.0 * weight_multiplier,
                hours * weight_multiplier,
            ]))?;
            dataset
                .targets
                .push(Axis(0), aview0(&(residual_differential.0 * weight_multiplier)))?;

            previous = log;
        }
        if dataset.nsamples() == 0 {
            bail!("empty dataset");
        }

        info!(n_samples = dataset.nsamples(), ?weight_mode, "estimating the battery efficiency…");
        let start_time = Instant::now();
        let regression = LinearRegression::new()
            .with_intercept(false)
            .fit(&dataset)
            .context("failed to fit a regression")?;
        info!(elapsed = ?start_time.elapsed(), "completed");

        let this = Self::builder()
            .charging(regression.params()[0])
            .discharging(-1.0 / regression.params()[1])
            .parasitic_load(Quantity(-regression.params()[2]))
            .n_samples(dataset.nsamples())
            .total_time(total_time)
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
            .add_row(vec![
                Cell::from("Total time"),
                Cell::from(format!("{:.1} days", self.total_time.as_seconds_f64() / 86400.0)),
            ])
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
