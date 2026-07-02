//! FoxESS [MiniQube][1] Modbus client.
//!
//! [1]: https://fox-ess.uk/miniqube/

mod metrics;
pub mod schedule;

use std::range::RangeInclusive;

use fennec_modbus::{
    contrib::{mini_qube, mini_qube::functions},
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
            .call::<functions::ReadDesignCapacity>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the design capacity")?
            .into();
        let state_of_health = self
            .0
            .call::<functions::ReadStateOfHealth>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the SoH")?
            .try_into()?;
        let state_of_charge = self
            .0
            .call::<functions::ReadStateOfCharge>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the SoC")?
            .try_into()?;
        let total_grid_export_energy = self
            .0
            .call::<functions::ReadTotalGridExportEnergy>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the total exported energy")?
            .into();
        let total_grid_import_energy = self
            .0
            .call::<functions::ReadTotalGridImportEnergy>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the total exported energy")?
            .into();
        let active_power = self
            .0
            .call::<functions::ReadTotalActivePower>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the active power")?
            .into();
        let eps_active_power = self
            .0
            .call::<functions::ReadEpsActivePower>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the EPS active power")?
            .into();
        // TODO: this wastes "minimum system SoC", introduce a custom type with just the two registers?
        let state_of_charge_settings = self
            .0
            .call::<functions::ReadStateOfChargeSettings>(Self::UNIT_ID, address::Const)
            .await
            .context("failed to read the state-of-charge settings")?;

        Ok(Metrics {
            state_of_charge,
            state_of_health,
            design_capacity,
            total_grid_flow: Flow {
                import: total_grid_import_energy,
                export: total_grid_export_energy,
            },
            allowed_soc: RangeInclusive {
                start: state_of_charge_settings.min_on_grid.try_into()?,
                last: state_of_charge_settings.max.try_into()?,
            },
            active_power,
            eps_active_power,
        })
    }

    /// Write the schedule slot to the battery and verify it.
    ///
    /// Note that MQ2200 does not support the "read/write multiple registers" operation,
    /// so this function actually performs three steps non-atomically:
    ///
    /// 1. Read the current slot.
    /// 2. If the current slot differs from the expected slot:
    ///     1. Write the expected slot.
    ///     2. Read the slot back and verify it matches the expected slot.
    #[instrument(skip_all, fields(index = index))]
    pub async fn write_schedule_slot(&self, index: u8, slot: mini_qube::schedule::Slot) -> Result {
        let address = address::Stride::new(index.into());
        let current_slot =
            self.0.call::<functions::ReadScheduleEntry>(Self::UNIT_ID, address).await?;
        if current_slot != slot {
            info!(
                start_time = %slot.start_time,
                end_time = %slot.end_time,
                to = ?slot.working_mode,
                from = ?current_slot.working_mode,
            );
            self.0
                .call::<functions::WriteScheduleEntry>(
                    Self::UNIT_ID,
                    write_multiple::Args::new(address, slot),
                )
                .await?;
            ensure!(
                self.0.call::<functions::ReadScheduleEntry>(Self::UNIT_ID, address).await? == slot
            );
        }
        Ok(())
    }
}
