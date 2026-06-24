use std::time::Duration;

use backon::{ConstantBuilder, Retryable};
use chrono::{DateTime, Days, Local, NaiveDate};

use crate::{
    Schedule,
    api::frank_energie,
    energy,
    energy::Flow,
    prelude::*,
    quantity::price::KilowattHourPrice,
};

#[derive(
    Copy, Clone, Hash, Eq, PartialEq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
pub enum Provider {
    /// Quarterly [Frank Energie](https://www.frankenergie.nl).
    #[serde(rename = "frank_energie_quarterly")]
    FrankEnergieQuarterly,

    /// Hourly [Frank Energie](https://www.frankenergie.nl).
    #[serde(rename = "frank_energie_hourly")]
    FrankEnergieHourly,
}

impl Provider {
    const BACKOFF: ConstantBuilder = ConstantBuilder::new().with_delay(Duration::from_secs(10));

    /// Fetch energy prices for up to 2 days since the specified timestamp.
    #[instrument(skip_all, fields(now = ?now))]
    pub async fn get_future_prices(
        self,
        now: DateTime<Local>,
    ) -> Result<Schedule<energy::Flow<KilowattHourPrice>>> {
        const ONE_DAY: Days = Days::new(1);

        // TODO: potentially, check for tomorrow's prices doesn't require fetching today's prices:
        let today = now.date_naive();
        let mut prices = self.get_prices(today).await?;
        ensure!(prices.len() != 0, "received empty price schedule for today");

        prices.extend({
            let tomorrow = today.checked_add_days(ONE_DAY).unwrap();
            self.get_prices(tomorrow).await?
        })?;

        info!(len = prices.len(), "fetched energy prices");
        prices.advance_to(now);
        Ok(prices)
    }

    /// Fetch energy prices for a single day.
    async fn get_prices(self, on: NaiveDate) -> Result<Schedule<Flow<KilowattHourPrice>>> {
        let resolution = match self {
            Self::FrankEnergieQuarterly => frank_energie::Resolution::Quarterly,
            Self::FrankEnergieHourly => frank_energie::Resolution::Hourly,
        };
        (|| async { frank_energie::Api::new(resolution)?.get_prices(on).await })
            .retry(Self::BACKOFF)
            .notify(log_retried_error)
            .await
    }
}
