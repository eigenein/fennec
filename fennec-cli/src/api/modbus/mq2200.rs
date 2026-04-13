//! FoxESS Modbus clients.

use fennec_modbus::tcp::UnitId;

use crate::{
    battery,
    prelude::*,
    quantity::{energy::DecawattHours, power::Watts, ratios::Percentage},
};

/// FoxESS MQ2200 Modbus client.
#[must_use]
pub struct MQ2200(fennec_modbus::tcp::tokio::Client<String>);

impl MQ2200 {
    pub fn new(address: String) -> Self {
        Self(fennec_modbus::tcp::tokio::Client::builder().endpoint(address).build())
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
        self.read_u16(46609).await.context("failed to read the minimum system SoC").map(Percentage)
    }

    async fn read_min_soc_on_grid(&self) -> Result<Percentage> {
        self.read_u16(46611).await.context("failed to read the minimum SoC on grid").map(Percentage)
    }

    async fn read_max_soc(&self) -> Result<Percentage> {
        self.read_u16(46610).await.context("failed to read the maximum SoC").map(Percentage)
    }

    async fn read_design_capacity(&self) -> Result<DecawattHours> {
        self.read_u16(37635).await.context("failed to read the design capacity").map(DecawattHours)
    }

    async fn read_state_of_charge(&self) -> Result<Percentage> {
        self.read_u16(39424).await.context("failed to read the SoC").map(Percentage)
    }

    async fn read_state_of_health(&self) -> Result<Percentage> {
        self.read_u16(37624).await.context("failed to read the SoH").map(Percentage)
    }

    /// Read total external active power.
    ///
    /// Positive means discharging, negative means charging.
    async fn read_active_power(&self) -> Result<Watts> {
        self.read_i32(39134).await.context("failed to read the active power").map(Into::into)
    }

    /// Read current EPS output power.
    async fn read_eps_active_power(&self) -> Result<Watts> {
        self.read_i32(39216).await.context("failed to read the EPS active power").map(Into::into)
    }

    async fn read_u16(&self, address: u16) -> Result<u16> {
        Ok(self.read_holding_registers::<1>(address).await.context("failed to read `u16`")?[0])
    }

    async fn read_i32(&self, address: u16) -> Result<i32> {
        let [high, low] =
            self.read_holding_registers::<2>(address).await.context("failed to read `u32`")?;
        let [high_1, high_0] = high.to_be_bytes();
        let [low_1, low_0] = low.to_be_bytes();
        Ok(i32::from_be_bytes([high_1, high_0, low_1, low_0]))
    }

    #[instrument(skip_all, fields(address = address))]
    async fn read_holding_registers<const N: usize>(&self, address: u16) -> Result<[u16; N]> {
        self.0
            .read_holding_registers_exact::<N>(UnitId::Significant(1), address)
            .await
            .context("Modbus error")
    }
}
