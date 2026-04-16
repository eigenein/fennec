//! FoxESS Modbus clients.

use fennec_modbus::{
    protocol::function::{ReadRegisters, read_registers, read_registers::Holding},
    tcp::UnitId,
};

use crate::{
    battery,
    prelude::*,
    quantity::{energy::DecawattHours, power::Watts, ratios::Percentage},
};

/// FoxESS MQ2200 Modbus client.
#[must_use]
pub struct MQ2200(fennec_modbus::tcp::tokio::Client<String>);

impl MQ2200 {
    const UNIT_ID: UnitId = UnitId::Significant(1);

    pub fn new(address: String) -> Self {
        Self(fennec_modbus::tcp::tokio::Client::new(address))
    }

    #[instrument(skip_all)]
    pub async fn read_state(&self) -> Result<battery::State> {
        // TODO: read these once and cache them:
        let design_capacity = self.read_design_capacity().await?;
        let health = self.read_state_of_health().await?;
        let min_system_charge = self.read_min_system_soc().await?;
        let min_soc_on_grid = self.read_min_soc_on_grid().await?;
        let max_soc = self.read_max_soc().await?;

        // Fast-changing values should be read next to each other with minimum delays:
        let charge = self.read_state_of_charge().await?;
        let active_power = self.read_active_power().await?;
        let eps_active_power = self.read_eps_active_power().await?;

        Ok(battery::State {
            design_capacity,
            charge,
            health,
            active_power,
            eps_active_power,
            min_system_charge,
            charge_range: (min_soc_on_grid..=max_soc).into(),
        })
    }

    async fn read_min_system_soc(&self) -> Result<Percentage> {
        Ok(Percentage(
            self.0
                .call::<ReadRegisters<Holding, Vec<u16>>>(
                    Self::UNIT_ID,
                    read_registers::Args::new(46609, 1)?,
                )
                .await
                .context("failed to read the minimum system SoC")?[0],
        ))
    }

    async fn read_min_soc_on_grid(&self) -> Result<Percentage> {
        Ok(Percentage(
            self.0
                .call::<ReadRegisters<Holding, Vec<u16>>>(
                    Self::UNIT_ID,
                    read_registers::Args::new(46611, 1)?,
                )
                .await
                .context("failed to read the minimum SoC on grid")?[0],
        ))
    }

    async fn read_max_soc(&self) -> Result<Percentage> {
        Ok(Percentage(
            self.0
                .call::<ReadRegisters<Holding, Vec<u16>>>(
                    Self::UNIT_ID,
                    read_registers::Args::new(46610, 1)?,
                )
                .await
                .context("failed to read the maximum SoC")?[0],
        ))
    }

    async fn read_design_capacity(&self) -> Result<DecawattHours> {
        Ok(DecawattHours(
            self.0
                .call::<ReadRegisters<Holding, Vec<u16>>>(
                    Self::UNIT_ID,
                    read_registers::Args::new(37635, 1)?,
                )
                .await
                .context("failed to read the design capacity")?[0],
        ))
    }

    async fn read_state_of_charge(&self) -> Result<Percentage> {
        Ok(Percentage(
            self.0
                .call::<ReadRegisters<Holding, Vec<u16>>>(
                    Self::UNIT_ID,
                    read_registers::Args::new(39424, 1)?,
                )
                .await
                .context("failed to read the SoC")?[0],
        ))
    }

    async fn read_state_of_health(&self) -> Result<Percentage> {
        Ok(Percentage(
            self.0
                .call::<ReadRegisters<Holding, Vec<u16>>>(
                    Self::UNIT_ID,
                    read_registers::Args::new(37624, 1)?,
                )
                .await
                .context("failed to read the SoH")?[0],
        ))
    }

    /// Read total external active power.
    ///
    /// Positive means discharging, negative means charging.
    async fn read_active_power(&self) -> Result<Watts> {
        Ok(Watts::from(
            self.0
                .call::<ReadRegisters<Holding, Vec<i32>>>(
                    Self::UNIT_ID,
                    read_registers::Args::new(39134, 1)?,
                )
                .await
                .context("failed to read the active power")?[0],
        ))
    }

    /// Read current EPS output power.
    async fn read_eps_active_power(&self) -> Result<Watts> {
        Ok(Watts::from(
            self.0
                .call::<ReadRegisters<Holding, Vec<i32>>>(
                    Self::UNIT_ID,
                    read_registers::Args::new(39216, 1)?,
                )
                .await
                .context("failed to read the EPS active power")?[0],
        ))
    }
}
