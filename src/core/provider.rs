use chrono::{DateTime, Days, Local, NaiveDate};
use ordered_float::OrderedFloat;

use crate::{
    api::{frank_energie, frank_energie::Resolution, next_energy},
    core::series::Point,
    prelude::*,
    quantity::{Quantity, interval::Interval, rate::KilowattHourRate},
};

#[derive(
    Copy, Clone, Hash, Eq, PartialEq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
pub enum Provider {
    /// [NextEnergy](https://www.nextenergy.nl).
    NextEnergy,

    /// Quarterly [Frank Energie](https://www.frankenergie.nl).
    FrankEnergieQuarterly,

    /// Hourly [Frank Energie](https://www.frankenergie.nl).
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

    #[instrument(skip_all)]
    pub fn get_upcoming_rates(
        self,
        since: DateTime<Local>,
    ) -> Result<Vec<Point<Interval, KilowattHourRate>>> {
        let mut rates = self.get_rates(since.date_naive())?;
        let next_date = since.date_naive().checked_add_days(Days::new(1)).unwrap();
        rates.extend(self.get_rates(next_date)?);
        rates.retain(|(time_range, _)| time_range.end > since);
        Ok(rates)
    }

    fn get_rates(self, on: NaiveDate) -> Result<Vec<Point<Interval, KilowattHourRate>>> {
        match self {
            Self::NextEnergy => next_energy::Api::new().get_rates(on),
            Self::FrankEnergieQuarterly => {
                frank_energie::Api::new(Resolution::Quarterly).get_rates(on)
            }
            Self::FrankEnergieHourly => frank_energie::Api::new(Resolution::Hourly).get_rates(on),
        }
    }
}
