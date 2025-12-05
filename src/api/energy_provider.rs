use std::ops::Range;

use async_trait::async_trait;
use chrono::{DateTime, Days, Local, NaiveDate};

use crate::{core::series::Point, prelude::*, quantity::rate::KilowattHourRate};

#[async_trait]
pub trait EnergyProvider: Sync {
    #[instrument(skip_all)]
    async fn get_upcoming_rates(
        &self,
        since: DateTime<Local>,
    ) -> Result<Vec<Point<Range<DateTime<Local>>, KilowattHourRate>>> {
        let mut rates = self.get_rates(since.date_naive()).await?;
        let next_date = since.date_naive().checked_add_days(Days::new(1)).unwrap();
        rates.extend(self.get_rates(next_date).await?);
        rates.retain(|(time_range, _)| time_range.end > since);
        Ok(rates)
    }

    async fn get_rates(
        &self,
        on: NaiveDate,
    ) -> Result<Vec<Point<Range<DateTime<Local>>, KilowattHourRate>>>;
}
