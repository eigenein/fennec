use std::{
    fmt::{Display, Formatter},
    time::Instant,
};

use chrono::{NaiveTime, TimeDelta};
use comfy_table::{Attribute, Cell, CellAlignment, Color, Table, modifiers, presets};
use futures_core::TryStream;
use futures_util::TryStreamExt;

use super::Balance;
use crate::{
    cli::battery::BatteryPowerLimits,
    core::{battery::WorkingMode, quantum::Quantum},
    db::power,
    ops::Integrator,
    prelude::*,
    quantity::{Zero, energy::WattHours, power::Watts, time::Hours},
};

#[must_use]
pub struct BalanceProfile {
    time_step: TimeDelta,

    /// Fallback global average power flow for when a specific time bucket power flow is not available.
    average: Balance<Watts>,

    /// Average power flow within the time bucket.
    buckets: Vec<Option<Balance<Watts>>>,
}

impl BalanceProfile {
    #[instrument(skip_all)]
    pub async fn try_estimate<T>(
        battery_power_limits: BatteryPowerLimits,
        bucket_time_step: TimeDelta,
        mut logs: T,
    ) -> Result<Self>
    where
        T: TryStream<Ok = power::Measurement, Error = Error> + Unpin,
    {
        info!("crunching consumption logs…");
        let start_time = Instant::now();

        let mut previous = logs.try_next().await?.context("empty consumption logs")?;

        let mut fallback = Integrator::<Balance<WattHours>>::new();
        let mut buckets = {
            let max_naive_time =
                NaiveTime::from_num_seconds_from_midnight_opt(86399, 999_999_999).unwrap();
            let max_bucket_index = bucket_time_step.index(max_naive_time).unwrap();
            vec![fallback; max_bucket_index + 1]
        };

        while let Some(next) = logs.try_next().await? {
            let time_delta = Hours::from(next.timestamp - previous.timestamp);
            let net_power = (next.net_power + previous.net_power) / 2.0;

            let part = Integrator {
                time: time_delta,
                value: Balance::new(battery_power_limits, net_power) * time_delta,
            };
            fallback += part;

            if previous.timestamp.date_naive() == next.timestamp.date_naive() {
                let previous_bucket = bucket_time_step.index(previous.timestamp.time()).unwrap();
                let next_bucket = bucket_time_step.index(next.timestamp.time()).unwrap();
                if next_bucket == previous_bucket {
                    buckets[next_bucket] += part;
                }
            }

            previous = next;
        }

        info!(elapsed = ?start_time.elapsed(), "done");
        Ok(Self {
            time_step: bucket_time_step,
            average: fallback
                .average()
                .context("no samples to calculate the average energy balance")?,
            buckets: buckets.into_iter().map(Integrator::average).collect(),
        })
    }

    pub fn on(&self, time: NaiveTime) -> Balance<Watts> {
        self.buckets
            .get(self.time_step.index(time).unwrap())
            .copied()
            .flatten()
            .unwrap_or(self.average)
    }
}

impl Display for BalanceProfile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec![
                Cell::new("Bucket").set_alignment(CellAlignment::Right),
                Cell::new("Start\ntime").set_alignment(CellAlignment::Right),
                Cell::new("Grid\nimport").set_alignment(CellAlignment::Right).fg(Color::Red),
                Cell::new("Grid\nexport").set_alignment(CellAlignment::Right),
                Cell::new("Battery\nimport")
                    .set_alignment(CellAlignment::Right)
                    .fg(WorkingMode::Charge.color()),
                Cell::new("Battery\nexport")
                    .set_alignment(CellAlignment::Right)
                    .fg(WorkingMode::Discharge.color()),
            ]);
        for (index, flow) in self.buckets.iter().enumerate() {
            let flow = flow.unwrap_or(self.average);

            #[expect(clippy::cast_possible_truncation)]
            #[expect(clippy::cast_possible_wrap)]
            table.add_row(vec![
                Cell::new(index).set_alignment(CellAlignment::Right).add_attribute(Attribute::Dim),
                Cell::new((NaiveTime::MIN + self.time_step * index as i32).format("%H:%M"))
                    .set_alignment(CellAlignment::Right),
                Cell::new(flow.grid.import)
                    .set_alignment(CellAlignment::Right)
                    .fg(if flow.grid.import > Watts::ZERO { Color::Red } else { Color::Reset }),
                Cell::new(flow.grid.export).set_alignment(CellAlignment::Right),
                Cell::new(flow.battery.import)
                    .fg(WorkingMode::Charge.color())
                    .set_alignment(CellAlignment::Right),
                Cell::new(flow.battery.export)
                    .fg(WorkingMode::Discharge.color())
                    .set_alignment(CellAlignment::Right),
            ]);
        }
        write!(f, "{table}")
    }
}
