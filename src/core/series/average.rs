use std::{
    iter::Sum,
    ops::{Add, Div},
};

use chrono::{TimeDelta, Timelike};
use itertools::Itertools;

use crate::core::series::SumValues;

impl<T> AverageHourly for T where T: ?Sized {}

pub trait AverageHourly {
    fn average_hourly<K, V>(
        self,
    ) -> [Option<<<V as Div<TimeDelta>>::Output as Div<f64>>::Output>; 24]
    where
        Self: Sized + Iterator<Item = (K, (V, TimeDelta))>,
        K: Timelike,
        V: Default + Add<V, Output = V> + Div<TimeDelta>,
        <V as Div<TimeDelta>>::Output:
            Copy + Sum + Div<f64, Output = <V as Div<TimeDelta>>::Output>,
    {
        let mut averages = [None; 24];
        self.chunk_by(|(index, _)| index.hour())
            .into_iter()
            .map(|(hour, points)| {
                let (value, time_delta) = points.fold(
                    (V::default(), TimeDelta::zero()),
                    |(value_sum, time_delta_sum), (_, (value, time_delta))| {
                        (value_sum + value, time_delta_sum + time_delta)
                    },
                );
                (hour, value / time_delta)
            })
            .into_group_map_by(|(hour, _)| *hour)
            .into_iter()
            .map(|(hour, points)| {
                #[allow(clippy::cast_precision_loss)]
                let len = points.len() as f64;

                (hour, points.into_iter().sum_values() / len)
            })
            .for_each(|(index, value)| averages[index as usize] = Some(value));
        averages
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveTime;

    use super::*;
    use crate::quantity::{energy::KilowattHours, power::Kilowatts};

    #[test]
    fn test_average_hourly() {
        let series = vec![
            // Single point:
            (
                NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
                (KilowattHours::from(10.0), TimeDelta::minutes(60)),
            ),
            // 2kWh for 20 mins + 4kWh for 40 mins that's 6kWh in 60 mins, so 6kW:
            (
                NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
                (KilowattHours::from(2.0), TimeDelta::minutes(20)),
            ),
            (
                NaiveTime::from_hms_opt(23, 20, 0).unwrap(),
                (KilowattHours::from(4.0), TimeDelta::minutes(40)),
            ),
        ];
        let averages = series.into_iter().average_hourly();
        assert_eq!(&averages[0..23], [None; 23]);
        assert_eq!(averages[23], Some(Kilowatts::from(8.0))); // (10 + 6) / 2
    }
}
