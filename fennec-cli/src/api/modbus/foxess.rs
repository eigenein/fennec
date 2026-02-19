//! FoxESS Modbus clients.

use crate::{
    api::modbus,
    core::battery,
    prelude::*,
    quantity::{energy::DecawattHours, ratios::Percentage},
};

#[must_use]
pub struct EnergyStateClients {
    pub state_of_charge: modbus::Client,
    pub state_of_health: modbus::Client,
    pub design_capacity: modbus::Client,
}

impl EnergyStateClients {
    /// Read the battery energy state.
    pub async fn read(&self) -> Result<battery::EnergyState> {
        Ok(battery::EnergyState {
            design_capacity: DecawattHours(self.design_capacity.read_value().await?.try_into()?),
            state_of_charge: Percentage(self.state_of_charge.read_value().await?.try_into()?),
            state_of_health: Percentage(self.state_of_health.read_value().await?.try_into()?),
        })
    }
}

#[must_use]
pub struct Clients {
    pub energy_state: EnergyStateClients,
    pub min_state_of_charge: modbus::Client,
    pub max_state_of_charge: modbus::Client,
}

impl Clients {
    /// Read the full battery state.
    pub async fn read(&self) -> Result<battery::FullState> {
        let min_state_of_charge =
            Percentage(self.min_state_of_charge.read_value().await?.try_into()?);
        let max_state_of_charge =
            Percentage(self.max_state_of_charge.read_value().await?.try_into()?);
        Ok(battery::FullState {
            energy: self.energy_state.read().await?,
            allowed_state_of_charge: (min_state_of_charge..=max_state_of_charge).into(),
        })
    }
}
