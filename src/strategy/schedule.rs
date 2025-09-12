use serde::{Deserialize, Serialize};

use crate::{prelude::*, strategy::WorkingMode};

/// Working mode hourly schedule for 24 hours.
///
/// N-th element defines the working mode for the time slot of N:00:00-N:59:59.
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Serialize,
    Deserialize,
    derive_more::From,
    derive_more::Index,
    derive_more::IntoIterator,
)]
pub struct WorkingModeSchedule([WorkingMode; 24]);

impl WorkingModeSchedule {
    /// Build a daily schedule by zipping together the timings and working modes.
    #[instrument(skip_all, fields(starting_hour = starting_hour), name = "Building the hourly schedule…")]
    pub fn from_working_modes(
        starting_hour: u32,
        working_modes: impl IntoIterator<Item = WorkingMode>,
    ) -> Self {
        let mut this = Self::default();
        for (hour, working_mode) in
            (starting_hour as usize..).zip(working_modes.into_iter().take(24))
        {
            let hour = hour % 24;
            debug!("Set", hour = hour.to_string(), working_mode = format!("{working_mode:?}"));
            this.0[hour] = working_mode;
        }
        this
    }

    /// Randomly mutate the schedule.
    pub fn mutate(&mut self) {
        const MODES: [WorkingMode; 4] = [
            WorkingMode::Maintaining,
            WorkingMode::Balancing,
            WorkingMode::Charging,
            WorkingMode::Discharging,
        ];
        self.0[fastrand::usize(0..24)] = fastrand::choice(MODES).unwrap();
        self.0[fastrand::usize(0..24)] = fastrand::choice(MODES).unwrap();
    }

    /// Iterate the schedule starting with the specified hour.
    pub fn iter(&self, starting_hour: usize) -> impl Iterator<Item = (usize, WorkingMode)> {
        (0..24).map(move |i| {
            let hour = (i + starting_hour) % 24;
            (hour, self.0[hour])
        })
    }
}
