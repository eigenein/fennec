use chrono::{DateTime, Days, Local, NaiveDate};
use ordered_float::OrderedFloat;

use crate::{
    api::{frank_energie, frank_energie::Resolution, next_energy},
    core::interval::Interval,
    prelude::*,
    quantity::{Quantity, rate::KilowattHourRate},
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
    pub const fn purchase_fee(self) -> KilowattHourRate {
        match self {
            Self::NextEnergy => Quantity(OrderedFloat(0.021)),

            Self::FrankEnergieQuarterly | Self::FrankEnergieHourly => {
                Quantity(OrderedFloat(0.0182))
            }
        }
    }

    pub fn get_upcoming_rates(
        self,
        since: DateTime<Local>,
    ) -> Result<Vec<(Interval, KilowattHourRate)>> {
        let mut rates = self.get_rates(since.date_naive())?;
        let next_date = since.date_naive().checked_add_days(Days::new(1)).unwrap();
        rates.extend(self.get_rates(next_date)?);
        rates.retain(|(time_range, _)| time_range.end > since);
        Ok(rates)
    }

    #[instrument(skip_all)]
    pub fn get_rates(self, on: NaiveDate) -> Result<Vec<(Interval, KilowattHourRate)>> {
        match self {
            Self::NextEnergy => next_energy::Api::new().get_rates(on),
            Self::FrankEnergieQuarterly => {
                frank_energie::Api::new(Resolution::Quarterly).get_rates(on)
            }
            Self::FrankEnergieHourly => frank_energie::Api::new(Resolution::Hourly).get_rates(on),
        }
    }
}
