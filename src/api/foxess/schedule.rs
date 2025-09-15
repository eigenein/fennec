use std::fmt::{Display, Formatter};

use chrono::Timelike;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{cli::BatteryArgs, prelude::*, strategy::Point, units::Kilowatts};

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct Schedule {
    #[serde_as(as = "serde_with::BoolFromInt")]
    #[serde(rename = "enable")]
    pub is_enabled: bool,

    pub groups: TimeSlotSequence,
}

#[serde_as]
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
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
    pub feed_power_watts: u32,

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

impl Display for StartTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

impl StartTime {
    pub const fn from_hour(hour: u32) -> Self {
        Self { hour, minute: 0 }
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
    pub const fn from_hour(hour_inclusive: u32) -> Self {
        // End time is exclusive, but FoxESS Cloud won't accept `00:00`…
        let (hour, minute) = if hour_inclusive == 23 { (23, 59) } else { (hour_inclusive + 1, 0) };
        Self { hour, minute }
    }
}

impl TimeSlot {
    pub fn trace(&self) {
        info!(
            "Schedule group",
            is_enabled = self.is_enabled,
            start_time = self.start_time.to_string(),
            end_time = self.end_time.to_string(),
            work_mode = format!("{:?}", self.working_mode),
            feed_power_watts = self.feed_power_watts.to_string(),
        );
    }
}

#[derive(Serialize, Deserialize, derive_more::Deref)]
pub struct TimeSlotSequence(Vec<TimeSlot>);

impl TimeSlotSequence {
    #[instrument(skip_all, name = "Building FoxESS time slots from the schedule…")]
    pub fn from_schedule(
        schedule: impl IntoIterator<Item = Point<crate::strategy::WorkingMode>>,
        battery_args: &BatteryArgs,
    ) -> Result<Self> {
        schedule
            .into_iter()
            .take(24) // Avoid collisions with the same hours next day.
            .chunk_by(|point| {
                // Group by date as well because we cannot have time slots like 22:00-02:00:
                (point.time.date_naive(), point.value)
            })
            .into_iter()
            .take(8) // FoxESS Cloud allows maximum of 8 schedule groups.
            .map(|((_, working_mode), group)| {
                // Convert into time slots with their respective working mode:
                (working_mode, group.map(|point| point.time).collect::<Vec<_>>())
            })
            .map(|(working_mode, timestamps)| {
                let feed_power = match working_mode {
                    crate::strategy::WorkingMode::Discharging => battery_args.discharging_power,
                    crate::strategy::WorkingMode::Idle => Kilowatts::ZERO,
                    _ => battery_args.charging_power,
                };
                let working_mode = match working_mode {
                    crate::strategy::WorkingMode::Charging | crate::strategy::WorkingMode::Idle => {
                        WorkingMode::ForceCharge
                    }
                    crate::strategy::WorkingMode::Discharging => WorkingMode::ForceDischarge,
                    crate::strategy::WorkingMode::Balancing => WorkingMode::SelfUse,
                };
                let time_slot = TimeSlot {
                    is_enabled: true,
                    start_time: StartTime::from_hour(timestamps.first().unwrap().hour()),
                    end_time: EndTime::from_hour(timestamps.last().unwrap().hour()),
                    max_soc: 100,
                    min_soc_on_grid: battery_args.min_soc_percent,
                    feed_soc: battery_args.min_soc_percent,
                    feed_power_watts: feed_power.into_watts_u32(),
                    working_mode,
                };
                info!(
                    "Time slot",
                    start_time = time_slot.start_time.to_string(),
                    end_time = time_slot.end_time.to_string(),
                    working_mode = format!("{working_mode:?}"),
                    feed_power_watts = time_slot.feed_power_watts.to_string(),
                );
                Ok(time_slot)
            })
            .collect::<Result<_>>()
            .map(Self)
    }

    pub fn trace(&self) {
        for time_slot in &self.0 {
            time_slot.trace();
        }
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

    /// Anyhow, the API does not accept this one for my battery.
    #[serde(rename = "Backup")]
    Backup,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_time_try_from_ok() -> Result {
        assert_eq!(StartTime::from_hour(2), StartTime { hour: 2, minute: 0 });
        Ok(())
    }

    #[test]
    fn test_end_time_try_from_non_last_hour_ok() -> Result {
        assert_eq!(EndTime::from_hour(1), EndTime { hour: 2, minute: 0 });
        Ok(())
    }

    #[test]
    fn test_end_time_try_from_last_hour_ok() -> Result {
        assert_eq!(EndTime::from_hour(23), EndTime { hour: 23, minute: 59 });
        Ok(())
    }
}
