use crate::{prelude::*, strategy::WorkingMode};

/// Working mode hourly schedule for 24 hours.
///
/// N-th element defines the working mode for the time slot of N:00:00-N:59:59.
#[derive(Copy, Clone, Debug, derive_more::From, derive_more::IntoIterator)]
pub struct WorkingModeSchedule<const N_HOURS: usize = 24>([WorkingMode; N_HOURS]);

impl<const N_HOURS: usize> TryFrom<crate::cache::WorkingModeSchedule>
    for WorkingModeSchedule<N_HOURS>
{
    type Error = Error;

    /// Convert from the cached schedule.
    fn try_from(schedule: crate::cache::WorkingModeSchedule) -> Result<Self> {
        schedule
            .into_iter()
            .map(WorkingMode::try_from)
            .collect::<Result<Vec<WorkingMode>, prost::UnknownEnumValue>>()?
            .try_into()
            .map_err(|original| anyhow!("invalid schedule: {original:?}"))
            .map(Self)
    }
}

impl<const N: usize> Default for WorkingModeSchedule<N> {
    fn default() -> Self {
        Self([WorkingMode::default(); N])
    }
}

impl<const N_HOURS: usize> WorkingModeSchedule<N_HOURS> {
    /// Build a daily schedule by zipping together the timings and working modes.
    #[instrument(skip_all, fields(starting_hour = starting_hour), name = "Building the hourly schedule…")]
    pub fn from_working_modes(
        starting_hour: u32,
        working_modes: impl IntoIterator<Item = WorkingMode>,
    ) -> Self {
        let mut this = Self::default();
        for (hour, working_mode) in
            (starting_hour as usize..).zip(working_modes.into_iter().take(N_HOURS))
        {
            let hour = hour % N_HOURS;
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
        self.0[fastrand::usize(0..N_HOURS)] = fastrand::choice(MODES).unwrap();
        self.0[fastrand::usize(0..N_HOURS)] = fastrand::choice(MODES).unwrap();
    }

    /// Iterate the schedule starting with the specified hour.
    pub fn iter(&self, starting_hour: usize) -> impl Iterator<Item = (usize, WorkingMode)> {
        (0..N_HOURS).map(move |i| {
            let hour = (i + starting_hour) % N_HOURS;
            (hour, self.0[hour])
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_working_modes() {
        let working_modes = [
            WorkingMode::Charging,    // index 1
            WorkingMode::Discharging, // index 2
            WorkingMode::Balancing,   // index 0
            WorkingMode::Maintaining, // overflow and must be ignored
        ];
        let schedule = WorkingModeSchedule::<3>::from_working_modes(1, working_modes);
        assert_eq!(
            schedule.0,
            [WorkingMode::Balancing, WorkingMode::Charging, WorkingMode::Discharging],
        );
    }

    #[test]
    fn test_iter() {
        let actual: Vec<_> = WorkingModeSchedule([
            WorkingMode::Charging,
            WorkingMode::Discharging,
            WorkingMode::Maintaining,
        ])
        .iter(1)
        .collect();
        assert_eq!(
            actual,
            [
                (1, WorkingMode::Discharging),
                (2, WorkingMode::Maintaining),
                (0, WorkingMode::Charging)
            ]
        );
    }
}
