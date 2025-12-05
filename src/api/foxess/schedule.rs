use std::{
    fmt::{Display, Formatter},
    iter::once,
};

use chrono::{DateTime, Local, TimeDelta, Timelike};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    cli::BatteryArgs,
    core::working_mode::WorkingMode as CoreWorkingMode,
    prelude::*,
    quantity::{
        power::{Kilowatts, Watts},
        time_range::TimeRange,
    },
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
    /// First minute of a day.
    const FIRST_MINUTE: Self = Self { hour: 0, minute: 0 };
}

impl From<DateTime<Local>> for StartTime {
    fn from(timestamp: DateTime<Local>) -> Self {
        Self { hour: timestamp.hour(), minute: timestamp.minute() }
    }
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
    /// Last minute of a day.
    const LAST_MINUTE: Self = Self { hour: 23, minute: 59 };
}

impl From<DateTime<Local>> for EndTime {
    fn from(timestamp: DateTime<Local>) -> Self {
        Self { hour: timestamp.hour(), minute: timestamp.minute() }
    }
}

#[derive(Serialize, Deserialize, derive_more::AsRef, derive_more::IntoIterator)]
pub struct TimeSlotSequence(#[into_iterator(ref)] Vec<TimeSlot>);

impl TimeSlotSequence {
    #[instrument(skip_all)]
    pub fn from_schedule(
        schedule: impl IntoIterator<Item = (TimeRange, CoreWorkingMode)>,
        since: DateTime<Local>,
        battery_args: &BatteryArgs,
    ) -> Result<Self> {
        let until_exclusive = since + TimeDelta::days(1);
        info!(%since, %until_exclusive, "Building a FoxESS schedule…");
        schedule
            .into_iter()
            .filter_map(|(time_span, working_mode)| {
                // We can only build a time slot sequence for 24 hours:
                // FIXME: extract and test:
                if time_span.contains(since) {
                    // Truncate the past:
                    Some((TimeRange::new(since, time_span.end), working_mode))
                } else if time_span.contains(until_exclusive) {
                    // Truncate the future:
                    Some((TimeRange::new(time_span.start, until_exclusive), working_mode))
                } else if since <= time_span.start && time_span.end <= until_exclusive {
                    // Actual time span:
                    Some((time_span, working_mode))
                } else {
                    // Irrelevant time span:
                    None
                }
            })
            .chunk_by(|(_, mode)| {
                // Group sequential time steps by the working mode:
                *mode
            })
            .into_iter()
            .flat_map(|(working_mode, time_spans)| -> Result<_> {
                // Compress the time spans:
                let time_spans = time_spans.into_iter().collect_vec();
                let time_span = TimeRange::new(
                    time_spans.first().unwrap().0.start,
                    time_spans.last().unwrap().0.end,
                );
                // And convert into FoxESS time slots:
                Ok(into_time_slots(time_span)
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
                // TODO: extract a method:
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

fn into_time_slots(time_span: TimeRange) -> impl Iterator<Item = Option<(StartTime, EndTime)>> {
    let start_time = StartTime::from(time_span.start);

    let end_time = EndTime::from(time_span.end);
    if end_time.hour == 0 && end_time.minute == 0 {
        // FoxESS intervals are half-open, but they won't accept 00:00 as end time 🤦:
        return once(Some((start_time, EndTime::LAST_MINUTE))).chain(once(None));
    }

    if time_span.start.date_naive() == time_span.end.date_naive() {
        once(Some((start_time, end_time))).chain(once(None))
    } else {
        // Split cross-day time spans because we cannot have time slots like 22:00-02:00:
        once(Some((start_time, EndTime::LAST_MINUTE)))
            .chain(once(Some((StartTime::FIRST_MINUTE, end_time))))
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn test_try_into_time_slots_ok() {
        let start_time = Local.with_ymd_and_hms(2025, 11, 17, 22, 15, 0).unwrap();
        let end_time = Local.with_ymd_and_hms(2025, 11, 17, 23, 15, 0).unwrap();
        let slots = into_time_slots(TimeRange::new(start_time, end_time)).flatten().collect_vec();
        assert_eq!(
            slots,
            vec![(StartTime { hour: 22, minute: 15 }, EndTime { hour: 23, minute: 15 })],
        );
    }

    #[test]
    fn test_try_into_time_slots_midnight_ok() {
        let start_time = Local.with_ymd_and_hms(2025, 11, 17, 22, 15, 0).unwrap();
        let end_time = Local.with_ymd_and_hms(2025, 11, 18, 0, 0, 0).unwrap();
        let slots = into_time_slots(TimeRange::new(start_time, end_time)).flatten().collect_vec();
        assert_eq!(
            slots,
            vec![(StartTime { hour: 22, minute: 15 }, EndTime { hour: 23, minute: 59 })],
        );
    }

    #[test]
    fn test_try_into_time_slots_cross_day_ok() {
        let start_time = Local.with_ymd_and_hms(2025, 11, 17, 22, 15, 0).unwrap();
        let end_time = Local.with_ymd_and_hms(2025, 11, 18, 1, 15, 0).unwrap();
        let slots = into_time_slots(TimeRange::new(start_time, end_time)).flatten().collect_vec();
        assert_eq!(
            slots,
            vec![
                (StartTime { hour: 22, minute: 15 }, EndTime { hour: 23, minute: 59 }),
                (StartTime { hour: 0, minute: 0 }, EndTime { hour: 1, minute: 15 })
            ],
        );
    }
}
