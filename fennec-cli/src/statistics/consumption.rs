use chrono::{Local, Timelike};
use comfy_table::{Cell, CellAlignment, Color, Table, modifiers, presets};
use futures_core::TryStream;
use futures_util::TryStreamExt;
use itertools::Itertools;

use crate::{
    db::consumption::LogEntry,
    prelude::*,
    quantity::power::Kilowatts,
    statistics::accumulator::EnergyAccumulator,
};

#[must_use]
#[derive(Copy, Clone)]
pub struct ConsumptionStatistics {
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
        let mut total = EnergyAccumulator::default();
        let mut hourly = [EnergyAccumulator::default(); 24];

        while let Some(current) = logs.try_next().await? {
            let delta = EnergyAccumulator {
                time: current.timestamp - previous.timestamp,
                value: current.pv_deficit - previous.pv_deficit,
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
            average_deficit: total.average_power().context("empty consumption logs")?,
            hourly: hourly
                .into_iter()
                .map(EnergyAccumulator::average_power)
                .collect_array()
                .unwrap(),
        })
    }

    pub fn on_hour(&self, hour: u32) -> Kilowatts {
        self.hourly[hour as usize].unwrap_or(self.average_deficit)
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
                Cell::from("Deficit").set_alignment(CellAlignment::Right),
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
