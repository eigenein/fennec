use chrono::{Local, TimeDelta, Timelike};
use comfy_table::{Attribute, Cell, CellAlignment, Color, Table, modifiers, presets};
use derive_more::AddAssign;
use futures_core::TryStream;
use futures_util::TryStreamExt;
use humantime::format_duration;
use itertools::Itertools;

use crate::{
    db::consumption::ConsumptionLog,
    prelude::*,
    quantity::{energy::KilowattHours, power::Kilowatts},
};

#[must_use]
#[derive(Copy, Clone)]
pub struct ConsumptionStatistics {
    total: Accumulator,
    average: Kilowatts,
    hourly: [Option<Kilowatts>; 24],
}

impl ConsumptionStatistics {
    #[instrument(skip_all)]
    pub async fn try_estimate<T>(mut logs: T) -> Result<Self>
    where
        T: TryStream<Ok = ConsumptionLog, Error = Error> + Unpin,
    {
        info!("crunching consumption logs…");

        let mut previous = logs.try_next().await?.context("empty consumption logs")?;
        let mut total = Accumulator::default();
        let mut hourly = [Accumulator::default(); 24];

        while let Some(current) = logs.try_next().await? {
            let delta = Accumulator {
                time: current.timestamp - previous.timestamp,
                consumption: current.net - previous.net,
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
            average: total.average_power().context("empty consumption logs")?,
            hourly: hourly.into_iter().map(Accumulator::average_power).collect_array().unwrap(),
        })
    }

    pub fn on_hour(&self, hour: u32) -> Kilowatts {
        self.hourly[hour as usize].unwrap_or(self.average)
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
                Cell::from("Consumption"),
            ])
            .add_row(vec![
                Cell::from(self.average).add_attribute(Attribute::Bold),
                Cell::from(format_duration(self.total.time.to_std().unwrap())),
                Cell::from(self.total.consumption),
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
                        Some(power) if *power > self.average => Color::Red,
                        Some(power) if *power < self.average => Color::Green,
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
    consumption: KilowattHours,
}

impl Default for Accumulator {
    fn default() -> Self {
        Self { time: TimeDelta::zero(), consumption: KilowattHours::ZERO }
    }
}

impl Accumulator {
    pub fn average_power(self) -> Option<Kilowatts> {
        if self.time.is_zero() { None } else { Some(self.consumption / self.time) }
    }
}
