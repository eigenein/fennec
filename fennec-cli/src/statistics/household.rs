use chrono::{DateTime, Local, TimeDelta, Timelike};
use itertools::Itertools;

use crate::{
    api::home_assistant::EnergyState,
    core::series::Deltas,
    db::state::HourlyStandByPower,
    quantity::energy::KilowattHours,
};

impl FromIterator<(DateTime<Local>, EnergyState)> for HourlyStandByPower {
    fn from_iter<T: IntoIterator<Item = (DateTime<Local>, EnergyState)>>(iterator: T) -> Self {
        let mut this = [None; 24];
        for (hour, mean_power) in iterator
            .into_iter()
            .map(|(timestamp, energy_state)| (timestamp, energy_state.net_consumption))
            .deltas()
            .filter(|(interval, _)| {
                // Filter out cross-hour values:
                (interval.start.date_naive() == interval.end.date_naive())
                    && (interval.start.hour() == interval.end.hour())
            })
            .into_group_map_by(|(interval, _)| interval.start.hour())
            .into_iter()
            .map(|(hour, values)| {
                let (total_time, total_energy) = values.into_iter().fold(
                    (TimeDelta::zero(), KilowattHours::ZERO),
                    |(total_time, total_energy), (interval, energy)| {
                        (total_time + (interval.end - interval.start), total_energy + energy)
                    },
                );
                (hour, total_energy / total_time)
            })
        {
            this[hour as usize] = Some(mean_power);
        }
        Self::from(this)
    }
}
