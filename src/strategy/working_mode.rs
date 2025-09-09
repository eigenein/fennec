use crate::prelude::*;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum WorkingMode {
    /// Forced charging on any power.
    Charging,

    /// Forced discharging, no matter the actual consumption.
    Discharging,

    /// Charge on excess PV power, discharge on insufficient PV power.
    #[allow(dead_code)]
    Balancing,

    /// Do not do anything.
    #[default]
    Maintain,
}

/// Working mode schedule for 24 hours.
///
/// N-th element defines the working mode for the time slot of N:00:00-N:59:59.
///
/// # Constant generic parameter
///
/// Number of hours in a day – values other than 24 are only used in the tests.
#[derive(
    Debug, derive_more::From, derive_more::IntoIterator, derive_more::AsRef, derive_more::Deref,
)]
pub struct WorkingModeHourlySchedule<const N: usize = 24>([WorkingMode; N]);

impl<const N: usize> Default for WorkingModeHourlySchedule<N> {
    fn default() -> Self {
        Self([WorkingMode::default(); N])
    }
}

impl<const N: usize> WorkingModeHourlySchedule<N> {
    /// Build a daily schedule by zipping together the timings and working modes.
    #[instrument(skip_all, fields(starting_hour = starting_hour), name = "Building the hourly schedule…")]
    pub fn from_working_modes(
        starting_hour: u32,
        working_modes: impl IntoIterator<Item = WorkingMode>,
    ) -> Self {
        let mut this = Self::default();
        for (hour, working_mode) in
            (starting_hour as usize..).zip(working_modes.into_iter().take(N))
        {
            let hour = hour % N;
            debug!("Set", hour = hour.to_string(), working_mode = format!("{working_mode:?}"));
            this.0[hour] = working_mode;
        }
        this
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_ok() {
        let working_modes = [
            WorkingMode::Charging,    // index 1
            WorkingMode::Discharging, // index 2
            WorkingMode::Discharging, // index 0
            WorkingMode::Maintain,    // overflow and must be ignored
        ];
        let schedule = WorkingModeHourlySchedule::<3>::from_working_modes(1, working_modes);
        assert_eq!(
            schedule.0,
            [WorkingMode::Discharging, WorkingMode::Charging, WorkingMode::Discharging]
        );
    }
}
