use ordered_float::OrderedFloat;

use crate::{
    api,
    api::{frank_energie, next_energy},
    quantity::{Quantity, rate::KilowattHourRate},
};

#[derive(
    Copy, Clone, Hash, Eq, PartialEq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
pub enum Provider {
    /// https://www.nextenergy.nl
    NextEnergy,

    /// https://www.frankenergie.nl
    FrankEnergieQuarterly,

    /// https://www.frankenergie.nl
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
}

impl From<Provider> for Box<dyn api::energy_provider::EnergyProvider> {
    fn from(provider: Provider) -> Self {
        match provider {
            Provider::NextEnergy => Box::new(next_energy::Api::new()),
            Provider::FrankEnergieQuarterly => {
                Box::new(frank_energie::Api::new(frank_energie::Resolution::Quarterly))
            }
            Provider::FrankEnergieHourly => {
                Box::new(frank_energie::Api::new(frank_energie::Resolution::Hourly))
            }
        }
    }
}
