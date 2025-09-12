use crate::strategy::WorkingMode;

#[derive(derive_more::IntoIterator, prost::Message)]
pub struct WorkingModeSchedule<const N_HOURS: usize = 24>(
    #[prost(enumeration = "WorkingMode", tag = "1", repeated)] Vec<i32>,
);

impl From<crate::strategy::WorkingModeSchedule> for WorkingModeSchedule {
    /// Convert from the schedule.
    fn from(schedule: crate::strategy::WorkingModeSchedule) -> Self {
        Self(schedule.into_iter().map(|working_mode| working_mode as i32).collect())
    }
}
