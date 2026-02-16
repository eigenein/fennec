use chrono::{Local, TimeDelta, Timelike};
use comfy_table::{Attribute, Cell, CellAlignment, Color, Table, modifiers, presets};
use derive_more::AddAssign;
use futures_core::TryStream;
use futures_util::TryStreamExt;
use itertools::Itertools;

use crate::{
    db::consumption::LogEntry,
    prelude::*,
    quantity::{energy::KilowattHours, power::Kilowatts},
};

#[must_use]
#[derive(Copy, Clone)]
pub struct ConsumptionStatistics {
    total: Accumulator,
    average_deficit: Kilowatts,
    hourly: [Option<Kilowatts>; 24],
}

impl ConsumptionStatistics {
    #[instrument(skip_all)]
    pub async fn try_estimate<T>(mut logs: T) -> Result<Self>
    where
        T: TryStream<Ok = LogEntry, Error = Error> + Unpin,
    {
        info!("crunching consumption logs…");

        let mut previous = logs.try_next().await?.context("empty consumption logs")?;
        let mut total = Accumulator::default();
        let mut hourly = [Accumulator::default(); 24];

        while let Some(current) = logs.try_next().await? {
            let delta = Accumulator {
                time: current.timestamp - previous.timestamp,
                deficit: current.pv_deficit - previous.pv_deficit,
            };
            total += delta;
            if current.timestamp.date_naive() == previous.timestamp.date_naive()
                && current.timestamp.hour() == previous.timestamp.hour()
            {
                let local_hour = current.timestamp.with_timezone(&Local).hour() as usize;
                hourly[local_hour] += delta;
            }
            previous = current;
        }

        Ok(Self {
            total,
            average_deficit: total.average_deficit_power().context("empty consumption logs")?,
            hourly: hourly
                .into_iter()
                .map(Accumulator::average_deficit_power)
                .collect_array()
                .unwrap(),
        })
    }

    pub fn on_hour(&self, hour: u32) -> Kilowatts {
        self.hourly[hour as usize].unwrap_or(self.average_deficit)
    }

    #[must_use]
    pub fn summary_table(&self) -> Table {
        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec![
                Cell::from("Average").add_attribute(Attribute::Bold),
                Cell::from("Time"),
                Cell::from("Deficit"),
            ])
            .add_row(vec![
                Cell::from(self.average_deficit).add_attribute(Attribute::Bold),
                Cell::from(format!("{:.1} days", self.total.time.as_seconds_f64() / 86400.0)),
                Cell::from(self.total.deficit),
            ]);
        table
    }

    #[must_use]
    pub fn hourly_table(&self) -> Table {
        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec![
                Cell::from("Hour"),
                Cell::from("Power").set_alignment(CellAlignment::Right),
            ]);
        for (hour, power) in self.hourly.iter().enumerate() {
            table.add_row(vec![
                Cell::new(hour),
                power
                    .map(Cell::new)
                    .unwrap_or_else(|| Cell::new("n/a"))
                    .set_alignment(CellAlignment::Right)
                    .fg(match power {
                        Some(power) if *power > self.average_deficit => Color::Red,
                        Some(power) if *power < self.average_deficit => Color::Green,
                        _ => Color::Reset,
                    }),
            ]);
        }
        table
    }
}

#[derive(Copy, Clone, AddAssign)]
struct Accumulator {
    time: TimeDelta,
    deficit: KilowattHours,
}

impl Default for Accumulator {
    fn default() -> Self {
        Self { time: TimeDelta::zero(), deficit: KilowattHours::ZERO }
    }
}

impl Accumulator {
    pub fn average_deficit_power(self) -> Option<Kilowatts> {
        if self.time.is_zero() { None } else { Some(self.deficit / self.time) }
    }
}
