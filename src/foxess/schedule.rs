use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    cli::BatteryPower,
    optimizer::working_mode::WorkingModeHourlySchedule,
    prelude::*,
    units::Watts,
};

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
    #[instrument(skip_all, name = "Building time slots from the schedule…")]
    pub fn from_schedule<const N: usize>(
        schedule: WorkingModeHourlySchedule<N>,
        battery_power: BatteryPower,
        minimum_soc: u32,
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
                let (end_hour, end_minute) = {
                    // End time is exclusive, but FoxESS Cloud won't accept `00:00`…
                    let last_hour = *hours.last().unwrap();
                    if last_hour == (N - 1) { (N - 1, 59) } else { (last_hour + 1, 0) }
                };
                let working_mode = working_mode.into();
                let feed_power = {
                    if working_mode == WorkingMode::ForceDischarge {
                        battery_power.discharging
                    } else {
                        battery_power.charging
                    }
                };
                let time_slot = TimeSlot {
                    is_enabled: true,
                    start_hour: u32::try_from(*hours.first().unwrap())?,
                    start_minute: 0,
                    end_hour: u32::try_from(end_hour)?,
                    end_minute,
                    max_soc: 100,
                    min_soc_on_grid: minimum_soc,
                    feed_soc: minimum_soc,
                    feed_power_watts: Watts::from(feed_power).try_into()?,
                    working_mode,
                };
                info!(
                    "Time slot",
                    start_time =
                        format!("{:02}:{:02}", time_slot.start_hour, time_slot.start_minute),
                    end_time = format!("{:02}:{:02}", time_slot.end_hour, time_slot.end_minute),
                    working_mode = format!("{working_mode:?}"),
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

impl From<crate::optimizer::working_mode::WorkingMode> for WorkingMode {
    fn from(working_mode: crate::optimizer::working_mode::WorkingMode) -> Self {
        match working_mode {
            crate::optimizer::working_mode::WorkingMode::Charging => Self::ForceCharge,
            crate::optimizer::working_mode::WorkingMode::Discharging => Self::ForceDischarge,
            crate::optimizer::working_mode::WorkingMode::SelfUse => Self::SelfUse,
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use crate::{
        cli::BatteryPower,
        foxess::{
            FoxEssTimeSlot,
            schedule::{TimeSlotSequence, WorkingMode as FoxEssWorkingMode},
        },
        optimizer::working_mode::WorkingMode,
        prelude::*,
        units::Kilowatts,
    };

    #[test]
    fn test_from_daily_schedule_ok() -> Result {
        let daily_schedule = [
            WorkingMode::Charging,
            WorkingMode::Charging,
            WorkingMode::Discharging,
            WorkingMode::SelfUse,
        ]
        .into();
        let time_slot_sequence = TimeSlotSequence::from_schedule(
            daily_schedule,
            BatteryPower { charging: Kilowatts(dec!(1.2)), discharging: Kilowatts(dec!(0.8)) },
            10,
        )?;
        assert_eq!(
            time_slot_sequence.0,
            [
                FoxEssTimeSlot {
                    is_enabled: true,
                    start_hour: 0,
                    start_minute: 0,
                    end_hour: 2,
                    end_minute: 0,
                    max_soc: 100,
                    min_soc_on_grid: 10,
                    feed_soc: 10,
                    feed_power_watts: 1200,
                    working_mode: FoxEssWorkingMode::ForceCharge,
                },
                FoxEssTimeSlot {
                    is_enabled: true,
                    start_hour: 2,
                    start_minute: 0,
                    end_hour: 3,
                    end_minute: 0,
                    max_soc: 100,
                    min_soc_on_grid: 10,
                    feed_soc: 10,
                    feed_power_watts: 800,
                    working_mode: FoxEssWorkingMode::ForceDischarge,
                },
                FoxEssTimeSlot {
                    is_enabled: true,
                    start_hour: 3,
                    start_minute: 0,
                    end_hour: 3,
                    end_minute: 59,
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
