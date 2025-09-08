use std::fmt::{Display, Formatter};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{cli::BatteryArgs, optimizer::WorkingModeHourlySchedule, prelude::*};

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

impl TryFrom<usize> for StartTime {
    type Error = Error;

    fn try_from(hour: usize) -> Result<Self> {
        Ok(Self { hour: u32::try_from(hour)?, minute: 0 })
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
    pub fn try_from<const N: usize>(hour_inclusive: usize) -> Result<Self> {
        // End time is exclusive, but FoxESS Cloud won't accept `00:00`…
        let (hour, minute) =
            if hour_inclusive == (N - 1) { (N - 1, 59) } else { (hour_inclusive + 1, 0) };
        Ok(Self { hour: u32::try_from(hour)?, minute })
    }
}

impl TimeSlot {
    pub fn trace(&self) {
        info!(
            "Schedule group",
            is_enabled = self.is_enabled.to_string(),
            start_time = self.start_time.to_string(),
            end_time = self.end_time.to_string(),
            work_mode = format!("{:?}", self.working_mode),
            feed_power_watts = self.feed_power_watts.to_string(),
        );
    }
}

#[derive(Serialize, Deserialize, derive_more::Deref)]
pub struct TimeSlotSequence(pub Vec<TimeSlot>);

impl TimeSlotSequence {
    #[instrument(skip_all, name = "Building FoxESS time slots from the schedule…")]
    pub fn from_schedule<const N: usize>(
        schedule: WorkingModeHourlySchedule<N>,
        battery_args: &BatteryArgs,
    ) -> Result<Self> {
        let chunks = schedule.into_iter().enumerate().chunk_by(|(_, working_mode)| *working_mode);
        let chunks: Vec<_> = chunks.into_iter().collect();
        info!("Grouped schedule into chunks", n_chunks = chunks.len().to_string());
        if chunks.len() > 8 {
            bail!("FoxESS Cloud allows maximum of 8 schedule groups, got {}", chunks.len());
        }
        chunks
            .into_iter()
            .map(|(working_mode, time_slots)| {
                let hours: Vec<_> = time_slots.map(|(hour, _)| hour).collect();
                let feed_power = match working_mode {
                    crate::optimizer::WorkingMode::Discharging => -battery_args.discharging_power,
                    _ => battery_args.charging_power,
                };
                let time_slot = TimeSlot {
                    is_enabled: true,
                    start_time: StartTime::try_from(*hours.first().unwrap())?,
                    end_time: EndTime::try_from::<N>(*hours.last().unwrap())?,
                    max_soc: 100,
                    min_soc_on_grid: battery_args.min_soc_percent,
                    feed_soc: battery_args.min_soc_percent,
                    feed_power_watts: feed_power.into_watts_u32(),
                    working_mode: working_mode.into(),
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

impl From<crate::optimizer::WorkingMode> for WorkingMode {
    fn from(working_mode: crate::optimizer::WorkingMode) -> Self {
        match working_mode {
            crate::optimizer::WorkingMode::Charging => Self::ForceCharge,
            crate::optimizer::WorkingMode::Discharging => Self::ForceDischarge,
            crate::optimizer::WorkingMode::Balancing => Self::SelfUse,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        cli::BatteryArgs,
        foxess::{
            FoxEssTimeSlot,
            schedule::{EndTime, StartTime, TimeSlotSequence, WorkingMode as FoxEssWorkingMode},
        },
        optimizer::WorkingMode,
        prelude::*,
        units::power::Kilowatts,
    };

    #[test]
    fn test_start_time_try_from_ok() -> Result {
        assert_eq!(StartTime::try_from(2)?, StartTime { hour: 2, minute: 0 });
        Ok(())
    }

    #[test]
    fn test_end_time_try_from_non_last_hour_ok() -> Result {
        assert_eq!(EndTime::try_from::<24>(1)?, EndTime { hour: 2, minute: 0 });
        Ok(())
    }

    #[test]
    fn test_end_time_try_from_last_hour_ok() -> Result {
        assert_eq!(EndTime::try_from::<24>(23)?, EndTime { hour: 23, minute: 59 });
        Ok(())
    }

    #[test]
    fn test_from_daily_schedule_ok() -> Result {
        let daily_schedule = [
            WorkingMode::Charging,
            WorkingMode::Charging,
            WorkingMode::Discharging,
            WorkingMode::Balancing,
        ]
        .into();
        let time_slot_sequence = TimeSlotSequence::from_schedule(
            daily_schedule,
            &BatteryArgs {
                charging_power: Kilowatts(1.2),
                discharging_power: Kilowatts(-0.8),
                charging_efficiency: 1.0,
                discharging_efficiency: 1.0,
                self_discharging_rate: 0.0,
                min_soc_percent: 10,
            },
        )?;
        assert_eq!(
            time_slot_sequence.0,
            [
                FoxEssTimeSlot {
                    is_enabled: true,
                    start_time: StartTime { hour: 0, minute: 0 },
                    end_time: EndTime { hour: 2, minute: 0 },
                    max_soc: 100,
                    min_soc_on_grid: 10,
                    feed_soc: 10,
                    feed_power_watts: 1200,
                    working_mode: FoxEssWorkingMode::ForceCharge,
                },
                FoxEssTimeSlot {
                    is_enabled: true,
                    start_time: StartTime { hour: 2, minute: 0 },
                    end_time: EndTime { hour: 3, minute: 0 },
                    max_soc: 100,
                    min_soc_on_grid: 10,
                    feed_soc: 10,
                    feed_power_watts: 800,
                    working_mode: FoxEssWorkingMode::ForceDischarge,
                },
                FoxEssTimeSlot {
                    is_enabled: true,
                    start_time: StartTime { hour: 3, minute: 0 },
                    end_time: EndTime { hour: 3, minute: 59 },
                    max_soc: 100,
                    min_soc_on_grid: 10,
                    feed_soc: 10,
                    feed_power_watts: 1200,
                    working_mode: FoxEssWorkingMode::SelfUse,
                },
            ]
        );
        Ok(())
    }
}
