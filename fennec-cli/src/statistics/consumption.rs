use chrono::{Local, Timelike};
use comfy_table::{Cell, CellAlignment, Color, Table, modifiers, presets};
use futures_core::TryStream;
use futures_util::TryStreamExt;
use itertools::Itertools;

use crate::{
    cli::battery::BatteryPowerLimits,
    db::consumption::LogEntry,
    prelude::*,
    quantity::power::Kilowatts,
    statistics::accumulator::EnergyAccumulator,
};

#[must_use]
pub struct ConsumptionStatistics {
    average_deficit: Kilowatts,
    hourly_deficit: [Option<Kilowatts>; 24],
}

impl ConsumptionStatistics {
    #[instrument(skip_all)]
    pub async fn try_estimate<T>(power_limits: BatteryPowerLimits, mut logs: T) -> Result<Self>
    where
        T: TryStream<Ok = LogEntry, Error = Error> + Unpin,
    {
        info!("crunching consumption logs…");

        let mut previous = logs.try_next().await?.context("empty consumption logs")?;
        let mut total_deficit_accumulator = EnergyAccumulator::default();
        let mut hourly_deficit_accumulators = [EnergyAccumulator::default(); 24];

        while let Some(next) = logs.try_next().await? {
            let time_delta = next.timestamp - previous.timestamp;
            let pv_deficit = next.pv_deficit - previous.pv_deficit;
            let delta = EnergyAccumulator { time_delta, value: pv_deficit };
            total_deficit_accumulator += delta;

            if next.same_hour_as(&previous) {
                let local_hour = next.timestamp.with_timezone(&Local).hour() as usize;
                hourly_deficit_accumulators[local_hour] += delta;
            }

            previous = next;
        }

        Ok(Self {
            average_deficit: total_deficit_accumulator
                .average_power()
                .context("empty consumption logs")?,
            hourly_deficit: hourly_deficit_accumulators
                .into_iter()
                .map(EnergyAccumulator::average_power)
                .collect_array()
                .unwrap(),
        })
    }

    pub fn deficit_on(&self, hour: u32) -> Kilowatts {
        self.hourly_deficit[hour as usize].unwrap_or(self.average_deficit)
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
        for (hour, power) in self.hourly_deficit.iter().enumerate() {
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
