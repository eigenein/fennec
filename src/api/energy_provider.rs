use std::ops::Range;

use chrono::{DateTime, Days, Local, NaiveDate};

use crate::{core::series::Point, prelude::*, quantity::rate::KilowattHourRate};

pub trait EnergyProvider {
    #[instrument(skip_all)]
    async fn get_upcoming_rates(
        &self,
        since: DateTime<Local>,
    ) -> Result<impl Iterator<Item = Point<Range<DateTime<Local>>, KilowattHourRate>>> {
        let next_date = since.date_naive().checked_add_days(Days::new(1)).unwrap();
        Ok(self
            .get_rates(since.date_naive())
            .await?
            .chain(self.get_rates(next_date).await?)
            .filter(move |(time_range, _)| time_range.end > since))
    }

    async fn get_rates(
        &self,
        on: NaiveDate,
    ) -> Result<impl Iterator<Item = Point<Range<DateTime<Local>>, KilowattHourRate>>>;
}
