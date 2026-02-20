use chrono::NaiveDate;

use crate::{
    api::{frank_energie, frank_energie::Resolution, next_energy},
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
    pub const fn purchase_fee(self) -> KilowattHourPrice {
        match self {
            Self::NextEnergy => KilowattHourPrice(0.021),
            Self::FrankEnergieQuarterly | Self::FrankEnergieHourly => KilowattHourPrice(0.0182),
        }
    }

    pub async fn get_prices(self, on: NaiveDate) -> Result<Vec<(Interval, KilowattHourPrice)>> {
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
