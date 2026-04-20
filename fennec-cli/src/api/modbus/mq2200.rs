//! FoxESS Modbus clients.

use std::array::from_fn;

use fennec_modbus::{
    contrib::{
        mq2200,
        mq2200::{ReadScheduleEntryBlock, WriteScheduleEntryBlock, schedule},
    },
    protocol::{address, function::write_multiple},
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

    #[instrument(skip_all)]
    pub async fn write_schedule(&self, schedule: &schedule::Full) -> Result {
        let blocks: [[schedule::Entry; schedule::N_ENTRIES_PER_BLOCK]; schedule::N_BLOCKS] =
            from_fn(|block_index| {
                from_fn(|entry_index| {
                    schedule[block_index * schedule::N_ENTRIES_PER_BLOCK + entry_index]
                })
            });

        for (i, block) in (0u16..).zip(blocks) {
            info!(i, "writing the schedule block…");
            let address = schedule::BlockIndex(i);

            self.0
                .call::<WriteScheduleEntryBlock>(
                    Self::UNIT_ID,
                    write_multiple::Args::new(address, block),
                )
                .await?;

            ensure!(self.0.call::<ReadScheduleEntryBlock>(Self::UNIT_ID, address).await? == block);
            info!(i, "verified");
        }

        info!("finished");
        Ok(())
    }

    async fn read_min_system_soc(&self) -> Result<Percentage> {
        self.0
            .call::<mq2200::ReadMinimumSystemStateOfCharge>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the minimum system SoC")
            .and_then(TryInto::try_into)
    }

    async fn read_min_soc_on_grid(&self) -> Result<Percentage> {
        self.0
            .call::<mq2200::ReadMinimumStateOfChargeOnGrid>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the minimum SoC on grid")
            .and_then(TryInto::try_into)
    }

    async fn read_max_soc(&self) -> Result<Percentage> {
        self.0
            .call::<mq2200::ReadMaximumStateOfCharge>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the maximum SoC")
            .and_then(TryInto::try_into)
    }

    async fn read_design_capacity(&self) -> Result<DecawattHours> {
        self.0
            .call::<mq2200::ReadDesignCapacity>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the design capacity")
            .map(Into::into)
    }

    async fn read_state_of_charge(&self) -> Result<Percentage> {
        self.0
            .call::<mq2200::ReadStateOfCharge>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the SoC")
            .and_then(TryInto::try_into)
    }

    async fn read_state_of_health(&self) -> Result<Percentage> {
        self.0
            .call::<mq2200::ReadStateOfHealth>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the SoH")
            .and_then(TryInto::try_into)
    }

    /// Read total external active power.
    ///
    /// Positive means discharging, negative means charging.
    async fn read_active_power(&self) -> Result<Watts> {
        self.0
            .call::<mq2200::ReadTotalActivePower>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the active power")
            .map(Into::into)
    }

    /// Read current EPS output power.
    async fn read_eps_active_power(&self) -> Result<Watts> {
        self.0
            .call::<mq2200::ReadEpsActivePower>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the EPS active power")
            .map(Into::into)
    }
}
