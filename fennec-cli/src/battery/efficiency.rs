use std::{
    fmt::{Display, Formatter},
    time::Instant,
};

use bon::bon;
use comfy_table::{Cell, CellAlignment, Color, Table, modifiers, presets};
use futures_util::TryStreamExt;
use linfa::{Dataset, dataset::Records, traits::Fit};
use linfa_linear::LinearRegression;
use ndarray::{Array1, Array2, Axis, Ix1, aview0, aview1};

use crate::{
    db::{Db, battery::Measurement},
    fmt::FormattedPercentage,
    prelude::*,
    quantity::{Zero, power::Watts, time::Hours},
};

#[must_use]
#[derive(Copy, Clone)]
pub struct Efficiency {
    pub parasitic_load: Watts,

    /// Charging efficiency, `0..=1`.
    pub charging: f64,

    /// Discharging efficiency, `0..=1`.
    pub discharging: f64,

    pub n_samples: usize,
}

impl Efficiency {
    pub const IDEAL: Self =
        Self { parasitic_load: Watts::ZERO, charging: 1.0, discharging: 1.0, n_samples: 0 };
}

#[bon]
impl Efficiency {
    #[builder]
    pub fn new(
        parasitic_load: Watts,
        charging: f64,
        discharging: f64,
        n_samples: usize,
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
        Ok(Self { parasitic_load, charging, discharging, n_samples })
    }
}

impl Efficiency {
    pub const fn round_trip(&self) -> f64 {
        self.charging * self.discharging
    }

    /// Residual energy sensor bias, assuming it's symmetrical in regard to charging and discharging.
    ///
    /// For now, only exists to correct the estimated degradation costs.
    pub fn sensor_bias(&self) -> f64 {
        (self.charging / self.discharging).sqrt()
    }

    #[instrument(skip_all)]
    pub async fn try_estimate(db: &Db) -> Result<Self> {
        let dataset = Self::read_dataset(db).await?;

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
            .build()?;
        println!("{this}");
        Ok(this)
    }

    #[instrument(skip_all)]
    async fn read_dataset(db: &Db) -> Result<Dataset<f64, f64, Ix1>> {
        let mut measurements = db.measurements::<Measurement>().await?;
        let mut previous = measurements.try_next().await?.context("empty battery log stream")?;
        let mut dataset = Dataset::new(Array2::zeros((0, 3)), Array1::zeros(0));

        info!("reading the battery logs…");
        while let Some(log) = measurements.try_next().await? {
            let imported_energy = log.import - previous.import;
            let exported_energy = log.main_export - previous.main_export;
            let time_delta = Hours::from(log.timestamp - previous.timestamp);
            let residual_differential = log.residual_energy - previous.residual_energy;

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
        Ok(dataset)
    }
}

impl Display for Efficiency {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec![Cell::new("Battery")])
            .add_row(vec![
                Cell::new("Samples"),
                Cell::new(self.n_samples).set_alignment(CellAlignment::Right),
            ])
            .add_row(vec![
                Cell::new("Charging").fg(Color::Green),
                Cell::new(FormattedPercentage(self.charging))
                    .set_alignment(CellAlignment::Right)
                    .fg(Color::Green),
            ])
            .add_row(vec![
                Cell::new("Discharging").fg(Color::Red),
                Cell::new(FormattedPercentage(self.discharging))
                    .set_alignment(CellAlignment::Right)
                    .fg(Color::Red),
            ])
            .add_row(vec![
                Cell::new("Sensor bias"),
                Cell::new(FormattedPercentage(self.sensor_bias()))
                    .set_alignment(CellAlignment::Right),
            ])
            .add_row(vec![
                Cell::new("Parasitic load").fg(Color::DarkYellow),
                Cell::new(self.parasitic_load)
                    .fg(Color::DarkYellow)
                    .set_alignment(CellAlignment::Right),
            ])
            .add_row(vec![
                Cell::new("Round trip").fg(Color::DarkYellow),
                Cell::new(FormattedPercentage(self.round_trip()))
                    .set_alignment(CellAlignment::Right)
                    .fg(Color::DarkYellow),
            ]);
        write!(f, "{table}")
    }
}
