use std::time::Duration;

use backon::{ConstantBuilder, Retryable};
use chrono::NaiveDate;

use crate::{
    Schedule,
    api::frank_energie,
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

    pub async fn get_prices(self, on: NaiveDate) -> Result<Schedule<Flow<KilowattHourPrice>>> {
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
