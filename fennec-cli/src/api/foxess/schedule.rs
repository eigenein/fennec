use std::{
    fmt::{Display, Formatter},
    iter::once,
};

use chrono::{DateTime, Local, TimeDelta, Timelike};
use comfy_table::{Cell, CellAlignment, Table, modifiers, presets};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    api::foxess::working_mode::WorkingMode,
    cli::battery::BatteryPowerLimits,
    core::working_mode::WorkingMode as CoreWorkingMode,
    ops::{Interval, RangeInclusive},
    prelude::*,
    quantity::{
        power::{Kilowatts, Watts},
        proportions::Percent,
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
    pub max_soc: Percent,

    /// The minimum SoC value of the offline battery (minimal safe SoC value?).
    #[expect(clippy::doc_markdown)]
    #[serde(rename = "minSocOnGrid")]
    pub min_soc_on_grid: Percent,

    /// Discharge SoC value (minimal safe SoC value?).
    #[expect(clippy::doc_markdown)]
    #[serde(rename = "fdSoc")]
    pub feed_soc: Percent,

    /// The maximum fdischarge power value.
    ///
    /// # Note
    ///
    /// For MQ2200, this also seems to be the force *charge* power.
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
        schedule: impl IntoIterator<Item = (Interval, CoreWorkingMode)>,
        since: DateTime<Local>,
        battery_power_limits: BatteryPowerLimits,
        allowed_state_of_charge: RangeInclusive<Percent>,
    ) -> Result<Self> {
        let until_exclusive = since + TimeDelta::days(1);
        info!(%since, %until_exclusive, "building a FoxESS schedule…");
        schedule
            .into_iter()
            .filter_map(|(interval, working_mode)| {
                // We can only build a time slot sequence for 24 hours:
                // FIXME: extract and test:
                if interval.contains(since) {
                    // This interval has already begun:
                    Some((interval.with_start(since), working_mode))
                } else if interval.contains(until_exclusive) {
                    // This interval runs into the next day:
                    Some((interval.with_end(until_exclusive), working_mode))
                } else if since <= interval.start && interval.end <= until_exclusive {
                    // Actual time span:
                    Some((interval, working_mode))
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
            .flat_map(|(working_mode, chunk)| -> Result<_> {
                // Compress the intervals:
                let interval = {
                    let mut chunk = chunk.into_iter();
                    let first = chunk.next().unwrap().0;
                    let last = chunk.last().map_or_else(|| first, |(last, _)| last);
                    Interval::from_std(first.start..last.end)
                };
                // And convert into FoxESS time slots:
                Ok(into_time_slots(interval)
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
                        (WorkingMode::ForcedCharge, Kilowatts::ZERO)
                    }
                    CoreWorkingMode::Backup => {
                        // FIXME: actually, it seems like «load priority» is more suitable here.
                        (WorkingMode::BatteryPriority, battery_power_limits.charging_power)
                    }
                    CoreWorkingMode::Charge => {
                        (WorkingMode::ForcedCharge, battery_power_limits.charging_power)
                    }
                    CoreWorkingMode::Balance => {
                        (WorkingMode::SelfUse, battery_power_limits.discharging_power)
                    }
                    CoreWorkingMode::Discharge => {
                        (WorkingMode::ForcedDischarge, battery_power_limits.discharging_power)
                    }
                };
                // TODO: extract a method:
                let time_slot = TimeSlot {
                    is_enabled: true,
                    start_time,
                    end_time,
                    max_soc: allowed_state_of_charge.max,
                    min_soc_on_grid: allowed_state_of_charge.min,
                    feed_soc: allowed_state_of_charge.min,
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

impl Display for &TimeSlotSequence {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec!["Start", "End", "Mode", "Feed power"]);
        for time_slot in &self.0 {
            table.add_row(vec![
                Cell::new(&time_slot.start_time),
                Cell::new(&time_slot.end_time),
                Cell::new(format!("{}", time_slot.working_mode)).fg(time_slot.working_mode.color()),
                Cell::new(time_slot.feed_power).set_alignment(CellAlignment::Right),
            ]);
        }
        write!(f, "{table}")
    }
}

fn into_time_slots(interval: Interval) -> impl Iterator<Item = Option<(StartTime, EndTime)>> {
    let start_time = StartTime::from(interval.start);

    let end_time = EndTime::from(interval.end);
    if end_time.hour == 0 && end_time.minute == 0 {
        // FoxESS intervals are half-open, but they won't accept 00:00 as end time 🤦:
        return once(Some((start_time, EndTime::LAST_MINUTE))).chain(once(None));
    }

    if interval.start.date_naive() == interval.end.date_naive() {
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
        let slots =
            into_time_slots(Interval::from_std(start_time..end_time)).flatten().collect_vec();
        assert_eq!(
            slots,
            vec![(StartTime { hour: 22, minute: 15 }, EndTime { hour: 23, minute: 15 })],
        );
    }

    #[test]
    fn test_try_into_time_slots_midnight_ok() {
        let start_time = Local.with_ymd_and_hms(2025, 11, 17, 22, 15, 0).unwrap();
        let end_time = Local.with_ymd_and_hms(2025, 11, 18, 0, 0, 0).unwrap();
        let slots =
            into_time_slots(Interval::from_std(start_time..end_time)).flatten().collect_vec();
        assert_eq!(
            slots,
            vec![(StartTime { hour: 22, minute: 15 }, EndTime { hour: 23, minute: 59 })],
        );
    }

    #[test]
    fn test_try_into_time_slots_cross_day_ok() {
        let start_time = Local.with_ymd_and_hms(2025, 11, 17, 22, 15, 0).unwrap();
        let end_time = Local.with_ymd_and_hms(2025, 11, 18, 1, 15, 0).unwrap();
        let slots =
            into_time_slots(Interval::from_std(start_time..end_time)).flatten().collect_vec();
        assert_eq!(
            slots,
            vec![
                (StartTime { hour: 22, minute: 15 }, EndTime { hour: 23, minute: 59 }),
                (StartTime { hour: 0, minute: 0 }, EndTime { hour: 1, minute: 15 })
            ],
        );
    }
}
