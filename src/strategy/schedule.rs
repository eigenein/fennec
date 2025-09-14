use std::fmt::Debug;

use crate::strategy::WorkingMode;

/// Schedule for 24 hours.
///
/// N-th element defines the working mode for the time slot of N:00:00-N:59:59.
#[derive(Clone, Copy, Debug)]
pub struct HourlySchedule<T = WorkingMode, const N_HOURS: usize = 24> {
    /// The hour of the 0-th slot.
    ///
    /// FIXME: make private.
    pub start_hour: usize,

    /// FIXME: make private.
    pub slots: [T; N_HOURS],
}

impl<T, const N_HOURS: usize> HourlySchedule<T, N_HOURS>
where
    T: Copy,
{
    pub const fn get(&self, hour: usize) -> T {
        self.slots[(hour + N_HOURS - self.start_hour) % N_HOURS]
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
        self.slots[fastrand::usize(0..N_HOURS)] = fastrand::choice(MODES).unwrap();
        self.slots[fastrand::usize(0..N_HOURS)] = fastrand::choice(MODES).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get() {
        let schedule = HourlySchedule { start_hour: 1, slots: [1, 2, 0] };
        assert_eq!(schedule.get(0), 0);
    }
}
