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
    T: Copy + Default,
{
    pub fn from_iter(start_hour: u32, iter_slots: impl IntoIterator<Item = T>) -> Self {
        let mut this = Self { start_hour: start_hour as usize, slots: [T::default(); N_HOURS] };
        // FIXME: there must be a simpler way:
        for (i, slot) in iter_slots.into_iter().take(N_HOURS).enumerate() {
            this.slots[i] = slot;
        }
        this
    }
}

impl<T, const N_HOURS: usize> HourlySchedule<T, N_HOURS>
where
    T: Copy,
{
    /// Iterate the schedule by hours.
    pub fn iter(&self) -> impl Iterator<Item = (usize, T)> {
        (self.start_hour..).map(|hour| hour % N_HOURS).zip(self.slots)
    }

    pub const fn get(&self, hour: usize) -> T {
        // FIXME: this expression repeats itself:
        self.slots[(hour + N_HOURS - self.start_hour) % N_HOURS]
    }
}

impl<T, const N_HOURS: usize> HourlySchedule<T, N_HOURS> {
    /// Rotate the schedule so that the slots would start at the specified hour.
    pub fn rotate_to(&mut self, start_hour: usize) {
        // FIXME: this expression repeats itself:
        self.slots.rotate_right((self.start_hour + N_HOURS - start_hour) % N_HOURS);
        self.start_hour = start_hour;
    }

    /// Convert into array which starts at the specified hour.
    pub fn into_array(mut self, start_hour: usize) -> [T; N_HOURS] {
        self.rotate_to(start_hour);
        self.slots
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
    fn test_from_iter() {
        let schedule = HourlySchedule::<i32, 3>::from_iter(1, [1, 2, 0, 42]);
        assert_eq!(schedule.start_hour, 1);
        assert_eq!(schedule.slots, [1, 2, 0]);
    }

    #[test]
    fn test_iter() {
        let schedule = HourlySchedule { start_hour: 1, slots: [1, 2, 0] };
        assert_eq!(schedule.iter().collect::<Vec<_>>(), [(1, 1), (2, 2), (0, 0)]);
    }

    #[test]
    fn test_rotate_to() {
        let mut schedule = HourlySchedule { start_hour: 1, slots: [1, 2, 0] };

        schedule.rotate_to(0);
        assert_eq!(schedule.start_hour, 0);
        assert_eq!(schedule.slots, [0, 1, 2]);

        schedule.rotate_to(2);
        assert_eq!(schedule.start_hour, 2);
        assert_eq!(schedule.slots, [2, 0, 1]);
    }

    #[test]
    fn test_into_array() {
        let schedule = HourlySchedule { start_hour: 1, slots: [1, 2, 0] };
        assert_eq!(schedule.into_array(0), [0, 1, 2]);
    }

    #[test]
    fn test_get() {
        let schedule = HourlySchedule { start_hour: 1, slots: [1, 2, 0] };
        assert_eq!(schedule.get(0), 0);
    }
}
