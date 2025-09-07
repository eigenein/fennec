use chrono::{NaiveDateTime, Timelike};

use crate::prelude::*;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum WorkingMode {
    Charging,

    Discharging,

    /// Charge on excess PV power, discharge on insufficient PV power.
    #[default]
    SelfUse,
}

/// Working mode schedule for 24 hours.
///
/// N-th element defines the working mode for the time slot of N:00:00-N:59:59.
#[derive(Debug, Default, derive_more::IntoIterator, derive_more::AsRef, derive_more::Deref)]
pub struct WorkingModeDailySchedule([WorkingMode; 24]);

impl WorkingModeDailySchedule {
    /// Build a daily schedule by zipping together the timings and working modes.
    pub fn zip(
        slot_starts: impl IntoIterator<Item = NaiveDateTime>,
        working_modes: impl IntoIterator<Item = WorkingMode>,
    ) -> Self {
        let mut this = Self::default();
        for (start_time, working_mode) in slot_starts.into_iter().zip(working_modes).take(24) {
            info!(
                "Set",
                start_time = start_time.to_string(),
                working_mode = format!("{working_mode:?}")
            );
            this.0[start_time.hour() as usize] = working_mode;
        }
        this
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeDelta};

    use super::*;

    #[test]
    fn test_zip_ok() {
        let schedule_start_time =
            NaiveDate::from_ymd_opt(2025, 9, 7).unwrap().and_hms_opt(22, 0, 0).unwrap();
        let slot_starts: Vec<_> =
            (0_i64..=24_i64).map(|hours| schedule_start_time + TimeDelta::hours(hours)).collect();
        let working_modes = [
            WorkingMode::Charging,    // 22:00-23:00
            WorkingMode::Discharging, // 23:00-00:00
            WorkingMode::Discharging, // 00:00-01:00
            WorkingMode::Discharging, // 01:00-02:00
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse,
            WorkingMode::SelfUse, // next day 22:00-23:00, must be ignored
        ];
        let schedule = WorkingModeDailySchedule::zip(slot_starts, working_modes);
        assert_eq!(
            schedule.0,
            [
                WorkingMode::Discharging,
                WorkingMode::Discharging,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::SelfUse,
                WorkingMode::Charging,
                WorkingMode::Discharging,
            ]
        );
    }
}
