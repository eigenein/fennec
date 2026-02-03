use chrono::{Local, TimeDelta, Timelike};
use derive_more::AddAssign;
use futures_core::TryStream;
use futures_util::TryStreamExt;
use itertools::Itertools;

use crate::{
    db::consumption::ConsumptionLog,
    prelude::*,
    quantity::{energy::KilowattHours, power::Kilowatts},
};

#[derive(Copy, Clone)]
pub struct ConsumptionStatistics {
    pub global_average: Kilowatts,
    pub hourly: [Option<Kilowatts>; 24],
}

impl ConsumptionStatistics {
    #[instrument(skip_all)]
    pub async fn try_estimate<T>(mut logs: T) -> Result<Self>
    where
        T: TryStream<Ok = ConsumptionLog, Error = Error> + Unpin,
    {
        info!("crunching consumption logs…");

        let mut previous = logs.try_next().await?.context("empty consumption logs")?;
        let mut global = Accumulator::default();
        let mut hourly = [Accumulator::default(); 24];

        while let Some(current) = logs.try_next().await? {
            let delta = Accumulator {
                time: current.timestamp - previous.timestamp,
                consumption: current.net - previous.net,
            };
            global += delta;
            if current.timestamp.date_naive() == previous.timestamp.date_naive()
                && current.timestamp.hour() == previous.timestamp.hour()
            {
                let local_hour = current.timestamp.with_timezone(&Local).hour() as usize;
                hourly[local_hour] += delta;
            }
            previous = current;
        }

        let this = Self {
            global_average: global.average_power().context("empty consumption logs")?,
            hourly: hourly.into_iter().map(Accumulator::average_power).collect_array().unwrap(),
        };
        info!(
            global_average = ?this.global_average,
            total_time = %humantime::format_duration(global.time.to_std()?),
            total_consumption = ?global.consumption,
            "estimated consumption profile",
        );
        Ok(this)
    }

    pub fn on_hour(&self, hour: u32) -> Kilowatts {
        self.hourly[hour as usize].unwrap_or(self.global_average)
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
