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
    battery::WorkingMode,
    cli::battery::BatteryPowerLimits,
    db::power,
    ops::{BucketAverage, BucketIntegrator, Integrator},
    prelude::*,
    quantity::{Quantum, Zero, energy::WattHours, power::Watts, time::Hours},
};

#[must_use]
pub struct Profile {
    time_step: TimeDelta,
    average_balance: BucketAverage<Balance<Watts>>,
    pub average_eps_power: Watts,
}

impl Profile {
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

        let mut eps_power_integrator = Integrator::<WattHours>::new();
        let mut balance_integrator = {
            let max_naive_time =
                NaiveTime::from_num_seconds_from_midnight_opt(86399, 999_999_999).unwrap();
            BucketIntegrator::new(bucket_time_step.index(max_naive_time).unwrap())
        };

        while let Some(next) = logs.try_next().await? {
            let time_delta = Hours::from(next.timestamp - previous.timestamp);

            let interval_balance = Integrator::trapezoid(
                time_delta,
                Balance::new(battery_power_limits, previous.net_deficit),
                Balance::new(battery_power_limits, next.net_deficit),
            );
            balance_integrator.total += interval_balance;

            eps_power_integrator +=
                Integrator::trapezoid(time_delta, previous.eps_active_power, next.eps_active_power);

            if previous.timestamp.date_naive() == next.timestamp.date_naive() {
                let previous_bucket = bucket_time_step.index(previous.timestamp.time()).unwrap();
                let next_bucket = bucket_time_step.index(next.timestamp.time()).unwrap();
                if next_bucket == previous_bucket {
                    balance_integrator.buckets[next_bucket] += interval_balance;
                }
            }

            previous = next;
        }

        let average_eps_power = eps_power_integrator.average().unwrap_or(Watts::ZERO);
        info!(?average_eps_power, elapsed = ?start_time.elapsed(), "done");
        Ok(Self {
            time_step: bucket_time_step,
            average_balance: balance_integrator.try_into()?,
            average_eps_power,
        })
    }

    pub fn average_balance_on(&self, time: NaiveTime) -> Balance<Watts> {
        self.average_balance[self.time_step.index(time).unwrap()]
    }
}

impl Display for Profile {
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
        for (index, balance) in self.average_balance.iter().copied().enumerate() {
            #[expect(clippy::cast_possible_truncation)]
            #[expect(clippy::cast_possible_wrap)]
            table.add_row(vec![
                Cell::new(index).set_alignment(CellAlignment::Right).add_attribute(Attribute::Dim),
                Cell::new((NaiveTime::MIN + self.time_step * index as i32).format("%H:%M"))
                    .set_alignment(CellAlignment::Right),
                Cell::new(balance.grid.import)
                    .set_alignment(CellAlignment::Right)
                    .fg(if balance.grid.import > Watts::ZERO { Color::Red } else { Color::Reset }),
                Cell::new(balance.grid.export).set_alignment(CellAlignment::Right),
                Cell::new(balance.battery.import)
                    .fg(WorkingMode::Charge.color())
                    .set_alignment(CellAlignment::Right),
                Cell::new(balance.battery.export)
                    .fg(WorkingMode::Discharge.color())
                    .set_alignment(CellAlignment::Right),
            ]);
        }
        write!(f, "{table}")
    }
}
