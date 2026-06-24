//! FoxESS [MiniQube][1] Modbus client.
//!
//! [1]: https://fox-ess.uk/miniqube/

mod metrics;
pub mod schedule;

use std::array::from_fn;

use fennec_modbus::{
    contrib::{
        mini_qube,
        mini_qube::{ReadScheduleEntryBlock, WriteScheduleEntryBlock},
    },
    protocol::{address, function::write_multiple},
    tcp::UnitId,
};

pub use self::metrics::Metrics;
use crate::{energy::Flow, prelude::*};

/// FoxESS MQ2200 Modbus client.
#[must_use]
pub struct Client(fennec_modbus::tcp::tokio::Client<String>);

impl Client {
    const UNIT_ID: UnitId = UnitId::Significant(1);

    pub fn new(address: String) -> Self {
        Self(fennec_modbus::tcp::tokio::Client::new(address))
    }

    #[instrument(skip_all)]
    pub async fn read_metrics(&self) -> Result<Metrics> {
        let design_capacity = self
            .0
            .call::<mini_qube::ReadDesignCapacity>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the design capacity")?
            .into();
        let state_of_health = self
            .0
            .call::<mini_qube::ReadStateOfHealth>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the SoH")?
            .try_into()?;
        let state_of_charge = self
            .0
            .call::<mini_qube::ReadStateOfCharge>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the SoC")?
            .try_into()?;
        let total_grid_export_energy = self
            .0
            .call::<mini_qube::ReadTotalGridExportEnergy>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the total exported energy")?
            .into();
        let total_grid_import_energy = self
            .0
            .call::<mini_qube::ReadTotalGridImportEnergy>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the total exported energy")?
            .into();
        let active_power = self
            .0
            .call::<mini_qube::ReadTotalActivePower>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the active power")?
            .into();
        let eps_active_power = self
            .0
            .call::<mini_qube::ReadEpsActivePower>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the EPS active power")?
            .into();

        // TODO: these two are only needed when optimizing:
        let min_soc = self
            .0
            .call::<mini_qube::ReadMinimumStateOfChargeOnGrid>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the min SoC")?
            .try_into()?;
        let max_soc = self
            .0
            .call::<mini_qube::ReadMaximumStateOfCharge>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the max SoC")?
            .try_into()?;

        Ok(Metrics {
            state_of_charge,
            state_of_health,
            design_capacity,
            total_grid_flow: Flow {
                import: total_grid_import_energy,
                export: total_grid_export_energy,
            },
            allowed_soc: (min_soc..=max_soc).into(),
            active_power,
            eps_active_power,
        })
    }

    #[instrument(skip_all)]
    pub async fn write_schedule(&self, schedule: &mini_qube::schedule::Full) -> Result {
        let blocks: [[mini_qube::schedule::Entry; mini_qube::schedule::N_ENTRIES_PER_BLOCK];
            mini_qube::schedule::N_BLOCKS] = from_fn(|block_index| {
            from_fn(|entry_index| {
                schedule[block_index * mini_qube::schedule::N_ENTRIES_PER_BLOCK + entry_index]
            })
        });

        for (i, block) in (0u16..).zip(blocks) {
            info!(i, "writing the schedule block…");
            let address = mini_qube::schedule::BlockIndex(i);

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
