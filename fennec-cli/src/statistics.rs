pub mod battery;
pub mod flow;
mod integrator;

use std::{
    fmt::{Display, Formatter},
    time::Instant,
};

use chrono::{Local, Timelike};
use comfy_table::{Cell, CellAlignment, Color, Table, modifiers, presets};
use futures_core::TryStream;
use futures_util::TryStreamExt;
use itertools::Itertools;

use crate::{
    cli::battery::BatteryPowerLimits,
    core::working_mode::WorkingMode,
    db::power,
    prelude::*,
    quantity::{energy::WattHours, power::Watts, time::Hours},
    statistics::{flow::SystemFlow, integrator::Integrator},
};

#[must_use]
pub struct FlowStatistics {
    /// Fallback global average power flow for when a specific hourly power flow is not available.
    fallback: SystemFlow<Watts>,

    /// Average hourly power flow.
    hourly: [Option<SystemFlow<Watts>>; 24],
}

impl FlowStatistics {
    #[instrument(skip_all)]
    pub async fn try_estimate<T>(
        battery_power_limits: BatteryPowerLimits,
        mut logs: T,
    ) -> Result<Self>
    where
        T: TryStream<Ok = power::Measurement, Error = Error> + Unpin,
    {
        info!("crunching consumption logs…");
        let start_time = Instant::now();

        let mut previous = logs.try_next().await?.context("empty consumption logs")?;

        let mut fallback = Integrator::<SystemFlow<WattHours>>::default();
        let mut hourly = [fallback; 24];

        while let Some(next) = logs.try_next().await? {
            let time_delta = Hours::from(next.timestamp - previous.timestamp);
            let net_power = (next.net_power + previous.net_power) / 2.0;

            let flows = Integrator {
                hours: time_delta,
                value: SystemFlow::new(battery_power_limits, net_power) * time_delta,
            };
            fallback += flows;

            if next.same_hour_as(&previous) {
                let local_hour = usize::try_from(next.timestamp.with_timezone(&Local).hour())?;
                hourly[local_hour] += flows;
            }

            previous = next;
        }

        info!(elapsed = ?start_time.elapsed(), "done");
        Ok(Self {
            fallback: fallback
                .average()
                .context("no samples to calculate the fallback power flow")?,
            hourly: hourly.into_iter().map(Integrator::average).collect_array().unwrap(),
        })
    }

    pub fn on_hour(&self, hour: u32) -> SystemFlow<Watts> {
        self.hourly[hour as usize].unwrap_or(self.fallback)
    }
}

impl Display for FlowStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec![
                Cell::new("Hour").set_alignment(CellAlignment::Right),
                Cell::new("Grid\nimport").set_alignment(CellAlignment::Right).fg(Color::Red),
                Cell::new("Grid\nexport").set_alignment(CellAlignment::Right),
                Cell::new("Battery\nimport")
                    .set_alignment(CellAlignment::Right)
                    .fg(WorkingMode::Charge.color()),
                Cell::new("Battery\nexport")
                    .set_alignment(CellAlignment::Right)
                    .fg(WorkingMode::Discharge.color()),
            ]);
        for (hour, flow) in self.hourly.iter().enumerate() {
            let flow = flow.unwrap_or(self.fallback);
            table.add_row(vec![
                Cell::new(hour),
                Cell::new(flow.grid.import)
                    .set_alignment(CellAlignment::Right)
                    .fg(if flow.grid.import > Watts::zero() { Color::Red } else { Color::Reset }),
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
