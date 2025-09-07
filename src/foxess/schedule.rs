use chrono::{NaiveDateTime, TimeDelta, Timelike};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{cli::BatteryPower, optimizer::BatteryPlan, prelude::*, units::Watts};

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct Schedule {
    #[serde_as(as = "serde_with::BoolFromInt")]
    #[serde(rename = "enable")]
    pub is_enabled: bool,

    pub groups: TimeSlotSequence,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct TimeSlot {
    #[serde_as(as = "serde_with::BoolFromInt")]
    #[serde(rename = "enable")]
    pub is_enabled: bool,

    #[serde(rename = "startHour")]
    pub start_hour: u32,

    #[serde(rename = "startMinute")]
    pub start_minute: u32,

    #[serde(rename = "endHour")]
    pub end_hour: u32,

    #[serde(rename = "endMinute")]
    pub end_minute: u32,

    #[serde(rename = "maxSoc")]
    pub max_soc: u32,

    /// The minimum SoC value of the offline battery (minimal safe SoC value?).
    #[allow(clippy::doc_markdown)]
    #[serde(rename = "minSocOnGrid")]
    pub min_soc_on_grid: u32,

    /// Discharge SoC value (minimal safe SoC value?).
    #[allow(clippy::doc_markdown)]
    #[serde(rename = "fdSoc")]
    pub feed_soc: u32,

    /// The maximum discharge power value (but also, maximum charge power?).
    #[serde(rename = "fdPwr")]
    pub feed_power_watts: u32,

    #[serde(rename = "workMode")]
    pub working_mode: WorkingMode,
}

impl TimeSlot {
    pub fn trace(&self) {
        info!(
            "Schedule group",
            is_enabled = self.is_enabled.to_string(),
            start_time = format!("{:02}:{:02}", self.start_hour, self.start_minute),
            end_time = format!("{:02}:{:02}", self.end_hour, self.end_minute),
            work_mode = format!("{:?}", self.working_mode),
            feed_power_watts = self.feed_power_watts.to_string(),
        );
    }
}

#[derive(Serialize, Deserialize, derive_more::Deref)]
pub struct TimeSlotSequence(pub Vec<TimeSlot>);

impl TimeSlotSequence {
    pub fn from_battery_plan(
        now: NaiveDateTime,
        battery_plan: BatteryPlan,
        battery_power: BatteryPower,
        minimum_soc: u32,
    ) -> Self {
        let chunks = battery_plan
            .0
            .into_iter()
            .chunk_by(|entry| (entry.mode, entry.hourly_rate.start_at.date()));
        let groups = chunks
            .into_iter()
            .filter_map(|((working_mode, _), entries)| {
                // TODO: damn unit-test this…
                let entries: Vec<_> = entries.collect();
                let start_time = entries.first().unwrap().hourly_rate.start_at;
                let end_time =
                    // FIXME: could use `TimeDelta::hours(1)`, except for when end time is `00:00`.
                    entries.last().unwrap().hourly_rate.start_at + TimeDelta::minutes(59);
                if (end_time > now) && (end_time <= now + TimeDelta::days(1)) {
                    Some(TimeSlot {
                        is_enabled: true,
                        start_hour: start_time.hour(),
                        start_minute: start_time.minute(),
                        end_hour: end_time.hour(),
                        end_minute: 59,
                        max_soc: 100,
                        min_soc_on_grid: minimum_soc,
                        feed_soc: minimum_soc,
                        feed_power_watts: Watts::from(battery_power.max()).try_into().unwrap(), // FIXME
                        working_mode,
                    })
                } else {
                    None
                }
            })
            .collect();
        Self(groups)
    }

    pub fn trace(&self) {
        for group in &self.0 {
            group.trace();
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingMode {
    #[serde(rename = "SelfUse")]
    SelfUse,

    #[serde(rename = "Feedin")]
    FeedIn,

    #[serde(rename = "Backup")]
    Backup,

    #[serde(rename = "ForceCharge")]
    ForceCharge,

    #[serde(rename = "ForceDischarge")]
    ForceDischarge,
}
