use std::{
    fmt::{Display, Formatter},
    iter::once,
    time::Instant,
};

use chrono::{Local, Timelike};
use comfy_table::{Cell, CellAlignment, Table, modifiers, presets};
use futures_core::TryStream;
use futures_util::TryStreamExt;
use itertools::Itertools;

use crate::{
    cli::battery::BatteryPowerLimits,
    core::working_mode::{WorkingMode, WorkingModeMap},
    db::power,
    prelude::*,
    quantity::{energy::KilowattHours, power::Kilowatts},
    statistics::{flow::SystemFlow, integrator::Integrator},
};

#[must_use]
pub struct ConsumptionStatistics {
    /// Fallback power flow for when a specific hourly power flow is not available.
    fallback: WorkingModeMap<SystemFlow<Kilowatts>>,

    /// Average hourly power flows per battery working mode.
    hourly: [Option<WorkingModeMap<SystemFlow<Kilowatts>>>; 24],
}

impl ConsumptionStatistics {
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

        let mut fallback = Integrator::<WorkingModeMap<SystemFlow<KilowattHours>>>::default();
        let mut hourly = [fallback; 24];

        while let Some(next) = logs.try_next().await? {
            let time_delta = next.timestamp - previous.timestamp;
            let net_power = (next.net_power + previous.net_power) / 2.0;

            let flows = Integrator {
                time_delta,
                value: WorkingModeMap::new(|working_mode| {
                    SystemFlow::new(battery_power_limits, working_mode, net_power)
                }) * time_delta,
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

    pub fn on_hour(&self, hour: u32) -> &WorkingModeMap<SystemFlow<Kilowatts>> {
        self.hourly[hour as usize].as_ref().unwrap_or(&self.fallback)
    }
}

impl Display for ConsumptionStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        const WORKING_MODES: [WorkingMode; 5] = [
            WorkingMode::Idle,
            WorkingMode::Harvest,
            WorkingMode::SelfUse,
            WorkingMode::Charge,
            WorkingMode::Discharge,
        ];

        let mut table = Table::new();
        let header = WORKING_MODES.into_iter().flat_map(|mode| {
            ["I", "E", "C", "D"].map(|title| {
                Cell::new(format!("{mode:.1}{title}"))
                    .set_alignment(CellAlignment::Right)
                    .fg(mode.color())
            })
        });
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(once(Cell::new("Hr").set_alignment(CellAlignment::Right)).chain(header));
        for (hour, flow_map) in self.hourly.iter().enumerate() {
            if let Some(flow_map) = flow_map {
                let row = WORKING_MODES.into_iter().flat_map(|mode| {
                    [
                        Cell::new(flow_map[mode].grid.import)
                            .fg(mode.color())
                            .set_alignment(CellAlignment::Right),
                        Cell::new(flow_map[mode].grid.export)
                            .fg(mode.color())
                            .set_alignment(CellAlignment::Right),
                        Cell::new(flow_map[mode].battery.import)
                            .fg(mode.color())
                            .set_alignment(CellAlignment::Right),
                        Cell::new(flow_map[mode].battery.export)
                            .fg(mode.color())
                            .set_alignment(CellAlignment::Right),
                    ]
                });
                table.add_row(once(Cell::new(hour)).chain(row));
            } else {
                table.add_row(vec![Cell::new(hour)]);
            }
        }
        write!(f, "{table}")
    }
}
