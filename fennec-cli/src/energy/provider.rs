use chrono::{NaiveDate, TimeDelta};

use crate::{
    api::{frank_energie, frank_energie::Resolution, next_energy},
    energy::Flow,
    ops::Interval,
    prelude::*,
    quantity::price::KilowattHourPrice,
};

#[derive(
    Copy, Clone, Hash, Eq, PartialEq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
pub enum Provider {
    /// [NextEnergy](https://www.nextenergy.nl).
    #[serde(rename = "next_energy")]
    NextEnergy,

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
            Self::NextEnergy | Self::FrankEnergieHourly => TimeDelta::hours(1),
            Self::FrankEnergieQuarterly => TimeDelta::minutes(15),
        }
    }

    pub async fn get_prices(
        self,
        on: NaiveDate,
    ) -> Result<Vec<(Interval, Flow<KilowattHourPrice>)>> {
        match self {
            Self::NextEnergy => next_energy::Api::new()?.get_prices(on).await,
            Self::FrankEnergieQuarterly => {
                frank_energie::Api::new(Resolution::Quarterly)?.get_prices(on).await
            }
            Self::FrankEnergieHourly => {
                frank_energie::Api::new(Resolution::Hourly)?.get_prices(on).await
            }
        }
    }
}
