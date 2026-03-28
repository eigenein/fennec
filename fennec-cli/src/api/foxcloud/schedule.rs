use std::{
    collections::BTreeMap,
    fmt::{Debug, Display, Formatter},
    iter::once,
};

use chrono::{DateTime, Local, TimeDelta, Timelike};
use comfy_table::{Attribute, Cell, CellAlignment, Color, Table, modifiers, presets};
use derive_more::{AsRef, IntoIterator};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    api::foxcloud::working_mode::WorkingMode,
    battery::WorkingMode as CoreWorkingMode,
    cli::battery::BatteryPowerLimits,
    ops::Interval,
    prelude::*,
    quantity::{Zero, power::Watts},
};

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct Schedule {
    #[serde_as(as = "serde_with::BoolFromInt")]
    #[serde(rename = "enable")]
    pub is_enabled: bool,

    #[serde(rename = "groups")]
    pub groups: Groups,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct Group {
    #[serde(flatten)]
    pub start_time: StartTime,

    #[serde(flatten)]
    pub end_time: EndTime,

    #[serde(rename = "workMode")]
    pub working_mode: WorkingMode,

    #[serde(rename = "extraParam")]
    pub extra: ExtraParameters,
}

#[derive(Serialize, Deserialize)]
pub struct ExtraParameters {
    #[serde(rename = "fdPwr")]
    pub feed_power: Watts,

    #[serde(flatten)]
    other: BTreeMap<String, serde_json::Value>,
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

#[derive(Serialize, Deserialize, AsRef, IntoIterator)]
pub struct Groups(#[into_iterator(ref)] pub Vec<Group>);

impl FromIterator<Group> for Groups {
    fn from_iter<T: IntoIterator<Item = Group>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Groups {
    /// Convert the schedule into FoxESS Cloud scheduler groups.
    #[instrument(skip_all)]
    pub fn from_schedule(
        schedule: impl IntoIterator<Item = (Interval, CoreWorkingMode)>,
        battery_power_limits: BatteryPowerLimits,
    ) -> Self {
        info!("building a FoxESS schedule…");
        schedule
            .into_iter()
            .scan(None, |schedule_end, (mut interval, working_mode)| {
                if let Some(schedule_end) = *schedule_end {
                    if interval.start >= schedule_end {
                        // Stop at first interval which starts outside the schedule.
                        return None;
                    }
                    if interval.end > schedule_end {
                        // Just in case the interval crosses the schedule boundary, cut it.
                        interval = interval.with_end(schedule_end);
                    }
                } else {
                    // On first interval, just define the schedule end time (exclusive boundary).
                    // FoxESS Cloud only accepts a 24-hour schedule.
                    *schedule_end = Some(interval.start + TimeDelta::days(1));
                }
                Some((interval, working_mode))
            })
            .flat_map(|(interval, working_mode)| {
                into_time_slots(interval)
                    .flatten()
                    .map(move |(start_time, end_time)| (working_mode, start_time, end_time))
            })
            .map(|(working_mode, start_time, end_time)| {
                let (working_mode, feed_power) = match working_mode {
                    CoreWorkingMode::Idle => {
                        // Forced charging at 0W is effectively idling:
                        (WorkingMode::ForceCharge, Watts::ZERO)
                    }
                    CoreWorkingMode::Harness => {
                        (WorkingMode::Backup, battery_power_limits.charging)
                    }
                    CoreWorkingMode::Charge => {
                        (WorkingMode::ForceCharge, battery_power_limits.charging)
                    }
                    CoreWorkingMode::SelfUse => {
                        (WorkingMode::SelfUse, battery_power_limits.discharging)
                    }
                    CoreWorkingMode::Discharge => {
                        (WorkingMode::ForceDischarge, battery_power_limits.discharging)
                    }
                    CoreWorkingMode::Compensate => {
                        (WorkingMode::FeedIn, battery_power_limits.discharging)
                    }
                };
                Group {
                    start_time,
                    end_time,
                    working_mode,
                    extra: ExtraParameters { feed_power, other: BTreeMap::new() },
                }
            })
            .collect()
    }
}

impl Display for &Groups {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec![
                "Start\ntime",
                "End\ntime",
                "Mode",
                "Feed\npower",
                "Other\nattributes",
            ]);
        for group in &self.0 {
            let other = group
                .extra
                .other
                .iter()
                .sorted_unstable_by_key(|(key, _)| key.as_str())
                .map(|(key, value)| format!("{key}={value}"))
                .join(" ");
            let foreground_color = if group.extra.feed_power == Watts::ZERO {
                Color::Reset
            } else {
                group.working_mode.color()
            };
            let attribute = match group.working_mode {
                _ if group.extra.feed_power == Watts::ZERO => Attribute::Dim,
                _ => Attribute::NormalIntensity,
            };
            table.add_row(vec![
                Cell::new(&group.start_time).fg(foreground_color).add_attribute(attribute),
                Cell::new(&group.end_time).fg(foreground_color).add_attribute(attribute),
                Cell::new(group.working_mode).fg(foreground_color).add_attribute(attribute),
                Cell::new(group.extra.feed_power)
                    .set_alignment(CellAlignment::Right)
                    .fg(foreground_color)
                    .add_attribute(attribute),
                Cell::new(other).fg(foreground_color).add_attribute(attribute),
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
