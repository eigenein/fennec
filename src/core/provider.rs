use crate::{
    api,
    api::{frank_energie, next_energy},
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
