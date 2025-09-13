use std::fmt::Debug;

use crate::{prelude::*, strategy::WorkingMode};

/// Schedule for 24 hours.
///
/// N-th element defines the working mode for the time slot of N:00:00-N:59:59.
#[derive(
    Clone,
    Copy,
    Debug,
    derive_more::From,
    derive_more::Index,
    derive_more::IndexMut,
    derive_more::Into,
    derive_more::IntoIterator,
)]
pub struct HourlySchedule<T = WorkingMode, const N_HOURS: usize = 24>([T; N_HOURS]);

impl<T, const N_HOURS: usize> Default for HourlySchedule<T, N_HOURS>
where
    T: Copy + Default,
{
    fn default() -> Self {
        Self([T::default(); N_HOURS])
    }
}

impl<T, const N_HOURS: usize> HourlySchedule<T, N_HOURS>
where
    T: Copy,
{
    /// Iterate the schedule starting with the specified hour.
    pub fn iter(&self, starting_hour: usize) -> impl Iterator<Item = (usize, T)> {
        (0..N_HOURS).map(move |i| {
            let hour = (i + starting_hour) % N_HOURS;
            (hour, self.0[hour])
        })
    }
}

impl<T, const N_HOURS: usize> HourlySchedule<T, N_HOURS>
where
    Self: Default,
    T: Debug,
{
    #[instrument(
        skip_all,
        fields(starting_hour = starting_hour),
        name = "Building the hourly schedule…"
    )]
    pub fn from_iter(starting_hour: u32, items: impl IntoIterator<Item = T>) -> Self {
        let mut this = Self::default();
        for (hour, item) in (starting_hour as usize..).zip(items).take(N_HOURS) {
            let hour = hour % N_HOURS;
            debug!("Set", hour, item = format!("{item:?}"));
            this.0[hour] = item;
        }
        this
    }
}

impl<const N_HOURS: usize> HourlySchedule<WorkingMode, N_HOURS> {
    /// Randomly mutate the schedule.
    pub fn mutate(&mut self) {
        const MODES: [WorkingMode; 4] = [
            WorkingMode::Retaining,
            WorkingMode::Balancing,
            WorkingMode::Charging,
            WorkingMode::Discharging,
        ];
        self.0[fastrand::usize(0..N_HOURS)] = fastrand::choice(MODES).unwrap();
        self.0[fastrand::usize(0..N_HOURS)] = fastrand::choice(MODES).unwrap();
    }
}
