use std::{
    fmt::{Display, Formatter},
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

impl Display for StartTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

impl StartTime {
    pub const MIDNIGHT: Self = Self { hour: 0, minute: 0 };

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
    pub const MIDNIGHT: Self = Self { hour: 23, minute: 59 };

    pub const fn from_hour(hour_inclusive: u32) -> Self {
        // End time is exclusive, but FoxESS Cloud won't accept `00:00`…
        let (hour, minute) = if hour_inclusive == 23 { (23, 59) } else { (hour_inclusive + 1, 0) };
        Self { hour, minute }
    }
}

#[derive(Serialize, Deserialize, derive_more::AsRef, derive_more::IntoIterator)]
pub struct TimeSlotSequence(#[into_iterator(ref)] Vec<TimeSlot>);

impl TimeSlotSequence {
    #[instrument(skip_all)]
    pub fn from_schedule<'a>(
        schedule: impl IntoIterator<Item = &'a (Range<DateTime<Local>>, CoreWorkingMode)>,
        battery_args: &BatteryArgs,
    ) -> Result<Self> {
        schedule
            .into_iter()
            .take(24) // Avoid collisions with the same hours next day.
            .chunk_by(|(time, mode)| {
                // Group by date as well because we cannot have time slots like 22:00-02:00:
                (time.start.date_naive(), *mode) // FIXME: make use of the time range.
            })
            .into_iter()
            .take(8) // FoxESS Cloud allows maximum of 8 schedule groups.
            .map(|((_, working_mode), group)| {
                // Convert into time slots with their respective working mode:
                (working_mode, group.map(|(time, _)| time).collect::<Vec<_>>())
            })
            .map(|(working_mode, timestamps)| {
                let (working_mode, feed_power) = match working_mode {
                    // Forced charging at 0W is effectively idling:
                    CoreWorkingMode::Idle => (WorkingMode::ForceCharge, Kilowatts::ZERO),

                    CoreWorkingMode::Backup => (WorkingMode::BackUp, battery_args.charging_power),

                    CoreWorkingMode::ChargeVerySlowly => {
                        (WorkingMode::ForceCharge, battery_args.charging_power * 0.25)
                    }

                    CoreWorkingMode::ChargeSlowly => {
                        (WorkingMode::ForceCharge, battery_args.charging_power * 0.5)
                    }

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
                    start_time: StartTime::from_hour(timestamps.first().unwrap().start.hour()), // FIXME: time range.
                    end_time: EndTime::from_hour(timestamps.last().unwrap().start.hour()), // FIXME: time range.
                    max_soc: 100,
                    min_soc_on_grid: battery_args.min_soc_percent,
                    feed_soc: battery_args.min_soc_percent,
                    feed_power: feed_power.into(),
                    working_mode,
                };
                Ok(time_slot)
            })
            .collect::<Result<_>>()
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
