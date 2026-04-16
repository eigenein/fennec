use std::{
    fmt::{Display, Formatter},
    time::Instant,
};

use chrono::{Local, NaiveTime, TimeDelta};
use comfy_table::{Attribute, Cell, CellAlignment, Color, Table, modifiers, presets};
use futures_core::TryStream;
use futures_util::TryStreamExt;

use super::Balance;
use crate::{
    battery,
    battery::{EfficiencyEstimator, WorkingMode},
    cli::battery::BatteryPowerLimits,
    db::power,
    ops::{BucketIntegrator, BucketMean, Integrator},
    prelude::*,
    quantity::{Quantum, Zero, power::Watts, time::Hours},
};

#[must_use]
pub struct Profile {
    pub average_eps_power: Watts,
    pub battery_efficiency: battery::Efficiency,

    time_step: TimeDelta,
    average_balance: BucketMean<Balance<Watts>>,
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

        let mut balance_integrator = {
            let max_naive_time =
                NaiveTime::from_num_seconds_from_midnight_opt(86399, 999_999_999).unwrap();
            BucketIntegrator::new(bucket_time_step.index(max_naive_time).unwrap())
        };
        let mut eps_power_integrator = Integrator::new();
        let mut parasitic_power_integrator = Integrator::new();
        let mut charging_efficiency_estimator = EfficiencyEstimator::new();
        let mut discharging_efficiency_estimator = EfficiencyEstimator::new();

        while let Some(current) = logs.try_next().await? {
            let duration = Hours::from(current.timestamp - previous.timestamp);

            {
                let sample = Integrator::trapezoid(
                    duration,
                    Balance::new(battery_power_limits, previous.net_deficit),
                    Balance::new(battery_power_limits, current.net_deficit),
                );
                balance_integrator.total += sample;

                let previous_timestamp = previous.timestamp.with_timezone(&Local);
                let current_timestamp = current.timestamp.with_timezone(&Local);

                if previous_timestamp.date_naive() == current_timestamp.date_naive() {
                    let previous_bucket =
                        bucket_time_step.index(previous_timestamp.time()).unwrap();
                    let next_bucket = bucket_time_step.index(current_timestamp.time()).unwrap();
                    if next_bucket == previous_bucket {
                        balance_integrator.buckets[next_bucket] += sample;
                    }
                }
            }

            eps_power_integrator += Integrator::trapezoid(
                duration,
                previous.eps_active_power,
                current.eps_active_power,
            );

            if let Some((previous, current)) = previous.battery.zip(current.battery) {
                let residual_energy_sample =
                    // The value sign here matches the active power sign, so charging is negative:
                    Integrator { weight: duration, value: previous.residual_energy - current.residual_energy };

                if previous.active_power == Watts::ZERO && current.active_power == Watts::ZERO {
                    parasitic_power_integrator += residual_energy_sample;
                } else if previous.active_power > Watts::ZERO && current.active_power > Watts::ZERO
                {
                    discharging_efficiency_estimator.push(
                        residual_energy_sample,
                        previous.active_power,
                        current.active_power,
                    );
                } else if previous.active_power < Watts::ZERO && current.active_power < Watts::ZERO
                {
                    charging_efficiency_estimator.push(
                        residual_energy_sample,
                        previous.active_power,
                        current.active_power,
                    );
                }
            }

            previous = current;
        }

        let average_eps_power = eps_power_integrator.mean().unwrap_or(Watts::ZERO);

        let parasitic_load = parasitic_power_integrator.mean().unwrap_or(Watts::ZERO);
        charging_efficiency_estimator.sub_assign_residual_energy(parasitic_load);
        discharging_efficiency_estimator.sub_assign_residual_energy(parasitic_load);
        let battery_efficiency = battery::Efficiency {
            charging: charging_efficiency_estimator.estimate().clamp(0.5, 1.5),
            discharging: (1.0 / discharging_efficiency_estimator.estimate()).clamp(0.5, 1.5),
            parasitic_load,
        };

        info!(
            battery_efficiency.charging,
            battery_efficiency.discharging,
            battery_round_trip_efficiency = battery_efficiency.round_trip(),
            ?average_eps_power,
            ?parasitic_load,
            elapsed = ?start_time.elapsed(),
            "done",
        );

        Ok(Self {
            time_step: bucket_time_step,
            average_balance: balance_integrator.try_into()?,
            average_eps_power,
            battery_efficiency,
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
