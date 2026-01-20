use chrono::DurationRound;
use fennec_quantities::rate::KilowattHourRate;

use crate::{
    core::{interval::Interval, provider::Provider},
    statistics::rates::ProviderStatistics,
};

pub trait Extend {
    fn extend_grid_rates(
        &mut self,
        provider: Provider,
        statistics: &ProviderStatistics,
        forecast_interval: Interval,
    );
}

impl Extend for Vec<(Interval, KilowattHourRate)> {
    fn extend_grid_rates(
        &mut self,
        provider: Provider,
        statistics: &ProviderStatistics,
        forecast_interval: Interval,
    ) {
        loop {
            let start_timestamp = match self.last() {
                Some((interval, _)) => interval.start + provider.rate_time_delta(),
                None => forecast_interval.start.duration_trunc(provider.rate_time_delta()).unwrap(),
            };
            if start_timestamp >= forecast_interval.end {
                break;
            }
            let Some(median_rate) = statistics.medians.get(&start_timestamp.time()) else {
                break;
            };
            self.push((
                Interval::new(start_timestamp, start_timestamp + provider.rate_time_delta()),
                *median_rate,
            ));
        }
    }
}
