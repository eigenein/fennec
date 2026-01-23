use chrono::{DateTime, Local, TimeDelta, Timelike};
use itertools::Itertools;
use serde::{Deserialize, Serialize, Serializer};

use crate::{
    api::home_assistant::EnergyState,
    core::series::Deltas,
    prelude::*,
    quantity::{energy::KilowattHours, power::Kilowatts},
};

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct EnergyStatistics {
    pub household: HouseholdParameters,
}

impl FromIterator<EnergyState> for EnergyStatistics {
    fn from_iter<T: IntoIterator<Item = EnergyState>>(iterator: T) -> Self {
        info!("Crunching numbers…");
        let series = iterator.into_iter().map(|state| (state.last_changed_at, state)).collect_vec();
        Self { household: series.into_iter().collect() }
    }
}

#[must_use]
#[derive(Default, Serialize, Deserialize)]
pub struct HouseholdParameters {
    #[serde(
        rename = "hourly_stand_by_power_kilowatts",
        serialize_with = "HouseholdParameters::serialize_hourly_stand_by_power"
    )]
    pub hourly_stand_by_power: [Option<Kilowatts>; 24],
}

impl FromIterator<(DateTime<Local>, EnergyState)> for HouseholdParameters {
    fn from_iter<T: IntoIterator<Item = (DateTime<Local>, EnergyState)>>(iterator: T) -> Self {
        let mut this = Self { hourly_stand_by_power: [None; 24] };
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
            this.hourly_stand_by_power[hour as usize] = Some(mean_power);
        }
        this
    }
}

impl HouseholdParameters {
    /// Serialize the array with the items rounded to watts.
    fn serialize_hourly_stand_by_power<S: Serializer>(
        hourly_stand_by_power: &[Option<Kilowatts>],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(
            hourly_stand_by_power.iter().map(|kilowatts| kilowatts.map(Kilowatts::round_to_watts)),
        )
    }
}
