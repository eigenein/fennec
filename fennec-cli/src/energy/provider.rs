use chrono::{NaiveDate, TimeDelta};

use crate::{
    api::frank_energie,
    energy::Flow,
    ops::Interval,
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
    pub const fn time_step(self) -> TimeDelta {
        match self {
            Self::FrankEnergieHourly => TimeDelta::hours(1),
            Self::FrankEnergieQuarterly => TimeDelta::minutes(15),
        }
    }

    pub async fn get_prices(
        self,
        on: NaiveDate,
    ) -> Result<Vec<(Interval, Flow<KilowattHourPrice>)>> {
        match self {
            Self::FrankEnergieQuarterly => {
                frank_energie::Api::new(frank_energie::Resolution::Quarterly)?.get_prices(on).await
            }
            Self::FrankEnergieHourly => {
                frank_energie::Api::new(frank_energie::Resolution::Hourly)?.get_prices(on).await
            }
        }
    }
}
