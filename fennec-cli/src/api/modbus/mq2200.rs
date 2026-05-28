//! FoxESS Modbus client.

use std::array::from_fn;

use chrono::Local;
use fennec_modbus::{
    contrib::{
        mq2200,
        mq2200::{ReadScheduleEntryBlock, WriteScheduleEntryBlock, schedule},
    },
    protocol::{address, function::write_multiple},
    tcp::UnitId,
};

use crate::{battery, energy::Flow, prelude::*};

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
        let design_capacity = self
            .0
            .call::<mq2200::ReadDesignCapacity>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the design capacity")?
            .into();
        let health = self
            .0
            .call::<mq2200::ReadStateOfHealth>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the SoH")?
            .try_into()?;

        // Fast-changing values should be read next to each other with minimum delays:
        let charge = self
            .0
            .call::<mq2200::ReadStateOfCharge>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the SoC")?
            .try_into()?;
        let active_power = self
            .0
            .call::<mq2200::ReadTotalActivePower>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the active power")?
            .into();
        let eps_active_power = self
            .0
            .call::<mq2200::ReadEpsActivePower>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the EPS active power")?
            .into();
        let total_grid_export_energy = self
            .0
            .call::<mq2200::ReadTotalGridExportEnergy>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the total exported energy")?
            .into();
        let total_grid_import_energy = self
            .0
            .call::<mq2200::ReadTotalGridImportEnergy>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the total exported energy")?
            .into();

        Ok(battery::State {
            timestamp: Local::now(),
            charge,
            health,
            design_capacity,
            active_power,
            eps_active_power,
            total_grid_flow: Flow {
                import: total_grid_import_energy,
                export: total_grid_export_energy,
            },
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
}
