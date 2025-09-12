use std::path::Path;

use prost::Message;

use crate::{prelude::*, strategy::WorkingMode};

#[derive(bon::Builder, Message)]
pub struct Cache {
    #[prost(message, required, tag = "1")]
    pub working_mode_schedule: WorkingModeSchedule,
}

impl Cache {
    #[instrument(skip_all, fields(path = %path.display()), name = "Reading cache…")]
    pub fn read_from(path: &Path) -> Self {
        Self::read_fallibly_from(path).unwrap_or_else(|error| {
            error!("Failed to load the cache", error = format!("{error:#}"));
            Self::default()
        })
    }

    fn read_fallibly_from(path: &Path) -> Result<Self> {
        if path.is_file() { Ok(Self::decode(&*std::fs::read(path)?)?) } else { Ok(Self::default()) }
    }

    #[instrument(skip_all, fields(path = %path.display()), name = "Writing cache…")]
    pub fn write_to(&self, path: &Path) {
        if let Err(error) = std::fs::write(path, self.encode_to_vec()) {
            error!("Failed to save the cache", error = format!("{error:#}"));
        }
    }
}

#[derive(Clone, derive_more::IntoIterator, Message)]
pub struct WorkingModeSchedule<const N_HOURS: usize = 24>(
    #[prost(enumeration = "WorkingMode", tag = "1", repeated)] Vec<i32>,
);

impl From<crate::strategy::WorkingModeSchedule> for WorkingModeSchedule {
    /// Convert from the schedule.
    fn from(schedule: crate::strategy::WorkingModeSchedule) -> Self {
        Self(schedule.into_iter().map(i32::from).collect())
    }
}
