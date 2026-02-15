use derive_more::From;
use tokio_modbus::client::Reader;

use crate::{
    api::modbus::battery_state::{BatteryEnergyState, BatterySettings, BatteryState},
    cli::battery::{BatteryEnergyStateRegisters, BatteryRegisters, BatterySettingRegisters},
    ops::RangeInclusive,
    prelude::*,
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
        Ok(BatterySettings {
            allowed_state_of_charge: RangeInclusive::from_std(
                min_state_of_charge..=max_state_of_charge,
            ),
        })
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
        Ok(value)
    }
}
