use derive_more::From;
use tokio_modbus::client::Reader;

use crate::{
    cli::battery::{BatteryEnergyStateRegisters, BatteryRegisters, BatterySettingRegisters},
    prelude::*,
    quantity::{
        energy::{DecawattHours, KilowattHours, MilliwattHours},
        proportions::Percent,
    },
};

#[must_use]
#[derive(From)]
pub struct Client(tokio_modbus::client::Context);

impl Client {
    #[instrument(skip_all)]
    pub async fn read_energy_state(
        &mut self,
        registers: BatteryEnergyStateRegisters,
    ) -> Result<BatteryEnergyState> {
        let design_capacity = self.read_holding_register(registers.design_capacity).await?.into();
        let state_of_charge = self.read_holding_register(registers.state_of_charge).await?.into();
        let state_of_health = self.read_holding_register(registers.state_of_health).await?.into();
        info!(?state_of_charge, ?state_of_health, ?design_capacity, "fetched the battery state");
        Ok(BatteryEnergyState { design_capacity, state_of_charge, state_of_health })
    }

    #[instrument(skip_all)]
    pub async fn read_battery_settings(
        &mut self,
        registers: BatterySettingRegisters,
    ) -> Result<BatterySettings> {
        let min_state_of_charge =
            self.read_holding_register(registers.min_state_of_charge_on_grid).await?.into();
        let max_state_of_charge =
            self.read_holding_register(registers.max_state_of_charge).await?.into();
        info!(?min_state_of_charge, ?max_state_of_charge, "fetched the battery settings");
        Ok(BatterySettings { min_state_of_charge, max_state_of_charge })
    }

    #[instrument(skip_all)]
    pub async fn read_battery_state(
        &mut self,
        registers: BatteryRegisters,
    ) -> Result<BatteryState> {
        Ok(BatteryState {
            energy: self.read_energy_state(registers.energy).await?,
            settings: self.read_battery_settings(registers.setting).await?,
        })
    }

    #[instrument(skip_all, level = "debug", fields(register = register))]
    async fn read_holding_register(&mut self, register: u16) -> Result<u16> {
        let value = self
            .0
            .read_holding_registers(register, 1)
            .await??
            .pop()
            .with_context(|| format!("nothing is read from the register #{register}"))?;
        debug!(value);
        Ok(value)
    }
}

#[must_use]
pub struct BatteryEnergyState {
    design_capacity: DecawattHours,
    state_of_charge: Percent,
    state_of_health: Percent,
}

impl BatteryEnergyState {
    /// Battery capacity corrected on the state of health.
    pub fn actual_capacity(&self) -> KilowattHours {
        KilowattHours::from(self.design_capacity) * self.state_of_health
    }

    /// Residual energy corrected on the state of health.
    pub fn residual(&self) -> KilowattHours {
        self.actual_capacity() * self.state_of_charge
    }

    /// Residual energy corrected on the state of health.
    pub fn residual_millis(&self) -> MilliwattHours {
        self.design_capacity * (self.state_of_health * self.state_of_charge)
    }
}

#[must_use]
pub struct BatterySettings {
    pub min_state_of_charge: Percent,
    pub max_state_of_charge: Percent,
}

#[must_use]
pub struct BatteryState {
    pub energy: BatteryEnergyState,
    pub settings: BatterySettings,
}

impl BatteryState {
    pub fn min_residual_energy(&self) -> KilowattHours {
        self.energy.actual_capacity() * self.settings.min_state_of_charge
    }

    pub fn max_residual_energy(&self) -> KilowattHours {
        self.energy.actual_capacity() * self.settings.max_state_of_charge
    }
}
