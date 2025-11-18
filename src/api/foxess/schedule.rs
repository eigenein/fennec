use std::{
    fmt::{Display, Formatter},
    iter::once,
    ops::Range,
};

use chrono::{DateTime, Local, Timelike};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    cli::BatteryArgs,
    core::working_mode::WorkingMode as CoreWorkingMode,
    prelude::*,
    quantity::power::{Kilowatts, Watts},
};

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct Schedule {
    #[serde_as(as = "serde_with::BoolFromInt")]
    #[serde(rename = "enable")]
    pub is_enabled: bool,

    #[serde(rename = "groups")]
    pub groups: TimeSlotSequence,
}

#[serde_as]
#[derive(Eq, PartialEq, Serialize, Deserialize)]
pub struct TimeSlot {
    #[serde_as(as = "serde_with::BoolFromInt")]
    #[serde(rename = "enable")]
    pub is_enabled: bool,

    #[serde(flatten)]
    pub start_time: StartTime,

    #[serde(flatten)]
    pub end_time: EndTime,

    #[serde(rename = "maxSoc")]
    pub max_soc: u32,

    /// The minimum SoC value of the offline battery (minimal safe SoC value?).
    #[expect(clippy::doc_markdown)]
    #[serde(rename = "minSocOnGrid")]
    pub min_soc_on_grid: u32,

    /// Discharge SoC value (minimal safe SoC value?).
    #[expect(clippy::doc_markdown)]
    #[serde(rename = "fdSoc")]
    pub feed_soc: u32,

    /// The maximum discharge power value (but also, maximum charge power?).
    #[serde(rename = "fdPwr")]
    pub feed_power: Watts,

    #[serde(rename = "workMode")]
    pub working_mode: WorkingMode,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StartTime {
    #[serde(rename = "startHour")]
    pub hour: u32,

    #[serde(rename = "startMinute")]
    pub minute: u32,
}

impl StartTime {
    const FIRST_MINUTE: Self = Self { hour: 0, minute: 0 };
}

impl Display for StartTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EndTime {
    #[serde(rename = "endHour")]
    pub hour: u32,

    #[serde(rename = "endMinute")]
    pub minute: u32,
}

impl Display for EndTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

impl EndTime {
    const LAST_MINUTE: Self = Self { hour: 23, minute: 59 };
}

#[derive(Serialize, Deserialize, derive_more::AsRef, derive_more::IntoIterator)]
pub struct TimeSlotSequence(#[into_iterator(ref)] Vec<TimeSlot>);

impl TimeSlotSequence {
    #[instrument(skip_all)]
    pub fn from_schedule(
        schedule: impl IntoIterator<Item = (Range<DateTime<Local>>, CoreWorkingMode)>,
        battery_args: &BatteryArgs,
    ) -> Result<Self> {
        schedule
            .into_iter()
            .chunk_by(|(_, mode)| *mode)
            .into_iter()
            .map(|(working_mode, time_spans)| {
                // Compress the time spans:
                let time_spans = time_spans.into_iter().collect_vec();
                (
                    working_mode,
                    time_spans.first().unwrap().0.start..time_spans.last().unwrap().0.end,
                )
            })
            .flat_map(|(working_mode, time_span)| -> Result<_> {
                Ok(try_into_time_slots(time_span)?
                    .flatten()
                    .map(move |(start_time, end_time)| (working_mode, start_time, end_time)))
            })
            .flatten()
            .take(
                // FoxESS Cloud allows maximum of 8 schedule groups, pity:
                8,
            )
            .map(|(working_mode, start_time, end_time)| {
                let (working_mode, feed_power) = match working_mode {
                    CoreWorkingMode::Idle => {
                        // Forced charging at 0W is effectively idling:
                        (WorkingMode::ForceCharge, Kilowatts::ZERO)
                    }
                    CoreWorkingMode::Backup => (WorkingMode::BackUp, battery_args.charging_power),
                    CoreWorkingMode::Charge => {
                        (WorkingMode::ForceCharge, battery_args.charging_power)
                    }
                    CoreWorkingMode::Balance => {
                        (WorkingMode::SelfUse, battery_args.discharging_power)
                    }
                    CoreWorkingMode::Discharge => {
                        (WorkingMode::ForceDischarge, battery_args.discharging_power)
                    }
                };

                let time_slot = TimeSlot {
                    is_enabled: true,
                    start_time,
                    end_time,
                    max_soc: 100,
                    min_soc_on_grid: battery_args.min_soc_percent,
                    feed_soc: battery_args.min_soc_percent,
                    feed_power: feed_power.into(),
                    working_mode,
                };
                Ok(time_slot)
            })
            .collect::<Result<_>>()
            .context("failed to compile a FoxESS schedule")
            .map(Self)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingMode {
    #[serde(rename = "SelfUse")]
    SelfUse,

    #[serde(rename = "Feedin")]
    FeedIn,

    #[serde(rename = "ForceCharge")]
    ForceCharge,

    #[serde(rename = "ForceDischarge")]
    ForceDischarge,

    #[serde(rename = "Backup")]
    BackUp,
}

fn try_into_time_slots(
    time_span: Range<DateTime<Local>>,
) -> Result<impl Iterator<Item = Option<(StartTime, EndTime)>>> {
    ensure!(time_span.start < time_span.end);

    ensure!(time_span.start.second() == 0 && time_span.start.nanosecond() == 0);
    let start_time = StartTime { hour: time_span.start.hour(), minute: time_span.start.minute() };

    ensure!(time_span.end.second() == 0 && time_span.end.nanosecond() == 0);
    let end_time = EndTime { hour: time_span.end.hour(), minute: time_span.end.minute() };
    if end_time.hour == 0 && end_time.minute == 0 {
        // FoxESS intervals are half-open, but they won't accept 00:00 as end time 🤦:
        return Ok(once(Some((start_time, EndTime::LAST_MINUTE))).chain(once(None)));
    }

    if time_span.start.date_naive() == time_span.end.date_naive() {
        Ok(once(Some((start_time, end_time))).chain(once(None)))
    } else {
        // Split cross-day time spans because we cannot have time slots like 22:00-02:00:
        Ok(once(Some((start_time, EndTime::LAST_MINUTE)))
            .chain(once(Some((StartTime::FIRST_MINUTE, end_time)))))
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn test_try_into_time_slots_ok() -> Result {
        let start_time = Local.with_ymd_and_hms(2025, 11, 17, 22, 15, 0).unwrap();
        let end_time = Local.with_ymd_and_hms(2025, 11, 17, 23, 15, 0).unwrap();
        let slots = try_into_time_slots(start_time..end_time)?.flatten().collect_vec();
        assert_eq!(
            slots,
            vec![(StartTime { hour: 22, minute: 15 }, EndTime { hour: 23, minute: 15 })],
        );
        Ok(())
    }

    #[test]
    fn test_try_into_time_slots_midnight_ok() -> Result {
        let start_time = Local.with_ymd_and_hms(2025, 11, 17, 22, 15, 0).unwrap();
        let end_time = Local.with_ymd_and_hms(2025, 11, 18, 0, 0, 0).unwrap();
        let slots = try_into_time_slots(start_time..end_time)?.flatten().collect_vec();
        assert_eq!(
            slots,
            vec![(StartTime { hour: 22, minute: 15 }, EndTime { hour: 23, minute: 59 })],
        );
        Ok(())
    }

    #[test]
    fn test_try_into_time_slots_cross_day_ok() -> Result {
        let start_time = Local.with_ymd_and_hms(2025, 11, 17, 22, 15, 0).unwrap();
        let end_time = Local.with_ymd_and_hms(2025, 11, 18, 1, 15, 0).unwrap();
        let slots = try_into_time_slots(start_time..end_time)?.flatten().collect_vec();
        assert_eq!(
            slots,
            vec![
                (StartTime { hour: 22, minute: 15 }, EndTime { hour: 23, minute: 59 }),
                (StartTime { hour: 0, minute: 0 }, EndTime { hour: 1, minute: 15 })
            ],
        );
        Ok(())
    }

    #[test]
    fn test_try_into_time_slots_error() {
        let start_time = Local.with_ymd_and_hms(2025, 11, 17, 22, 15, 0).unwrap();
        let end_time = Local.with_ymd_and_hms(2025, 11, 16, 23, 15, 0).unwrap();
        assert!(try_into_time_slots(start_time..end_time).is_err());
    }
}
