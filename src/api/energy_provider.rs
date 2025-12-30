use chrono::{DateTime, Days, Local, NaiveDate};

use crate::{
    core::series::Point,
    prelude::*,
    quantity::{interval::Interval, rate::KilowattHourRate},
};

/// TODO: merge into the enum.
pub trait EnergyProvider {
    #[instrument(skip_all)]
    fn get_upcoming_rates(
        &self,
        since: DateTime<Local>,
    ) -> Result<Vec<Point<Interval, KilowattHourRate>>> {
        let mut rates = self.get_rates(since.date_naive())?;
        let next_date = since.date_naive().checked_add_days(Days::new(1)).unwrap();
        rates.extend(self.get_rates(next_date)?);
        rates.retain(|(time_range, _)| time_range.end > since);
        Ok(rates)
    }

    fn get_rates(&self, on: NaiveDate) -> Result<Vec<Point<Interval, KilowattHourRate>>>;
}
