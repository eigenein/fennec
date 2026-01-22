use tokio_modbus::{
    Slave,
    client::{Reader, tcp::attach_slave},
};

use crate::{
    cli::{
        BatteryConnectionArgs,
        BatteryEnergyStateRegisters,
        BatteryRegisters,
        BatterySettingRegisters,
    },
    prelude::*,
    quantity::{Quantity, energy::KilowattHours},
};

#[must_use]
pub struct Client(tokio_modbus::client::Context);

impl Client {
    #[instrument(skip_all, fields(host = args.host, port = args.port, slave_id = args.slave_id))]
    pub async fn connect(args: &BatteryConnectionArgs) -> Result<Self> {
        info!("Connecting to the battery…");
        let tcp_stream = tokio::net::TcpStream::connect((args.host.as_str(), args.port))
            .await
            .context("failed to connect to the battery")?;
        Ok(Self(attach_slave(tcp_stream, Slave(args.slave_id))))
    }

    #[instrument(skip_all)]
    pub async fn read_energy_state(
        &mut self,
        registers: BatteryEnergyStateRegisters,
    ) -> Result<BatteryEnergyState> {
        info!("Reading the battery state…");
        let design_capacity = KilowattHours::from(
            // Stored in decawatts:
            0.01 * f64::from(self.read_holding_register(registers.design_capacity).await?),
        );
        let state_of_charge =
            0.01 * f64::from(self.read_holding_register(registers.state_of_charge).await?);
        let state_of_health =
            0.01 * f64::from(self.read_holding_register(registers.state_of_health).await?);
        Ok(BatteryEnergyState { design_capacity, state_of_charge, state_of_health })
    }

    #[instrument(skip_all)]
    pub async fn read_battery_settings(
        &mut self,
        registers: BatterySettingRegisters,
    ) -> Result<BatterySettings> {
        info!("Reading the battery settings…");
        let min_state_of_charge_percent =
            self.read_holding_register(registers.min_state_of_charge_on_grid).await?;
        let min_state_of_charge = 0.01 * f64::from(min_state_of_charge_percent);
        let max_state_of_charge_percent =
            self.read_holding_register(registers.max_state_of_charge).await?;
        let max_state_of_charge = 0.01 * f64::from(max_state_of_charge_percent);
        Ok(BatterySettings {
            min_state_of_charge_percent,
            min_state_of_charge,
            max_state_of_charge_percent,
            max_state_of_charge,
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

    #[instrument(skip_all, fields(register = register), ret)]
    async fn read_holding_register(&mut self, register: u16) -> Result<u16> {
        self.0
            .read_holding_registers(register, 1)
            .await??
            .pop()
            .with_context(|| format!("nothing is read from the register #{register}"))
    }
}

#[must_use]
pub struct BatteryEnergyState {
    design_capacity: KilowattHours,
    state_of_charge: f64,
    state_of_health: f64,
}

impl BatteryEnergyState {
    /// Battery capacity corrected on the state of health.
    pub const fn actual_capacity(&self) -> KilowattHours {
        Quantity(self.design_capacity.0 * self.state_of_health)
    }

    /// Residual energy corrected on the state of health.
    pub const fn residual(&self) -> KilowattHours {
        Quantity(self.actual_capacity().0 * self.state_of_charge)
    }
}

#[must_use]
pub struct BatterySettings {
    pub min_state_of_charge_percent: u16,
    pub min_state_of_charge: f64,

    pub max_state_of_charge_percent: u16,
    pub max_state_of_charge: f64,
}

#[must_use]
pub struct BatteryState {
    pub energy: BatteryEnergyState,
    pub settings: BatterySettings,
}

impl BatteryState {
    pub const fn min_residual_energy(&self) -> KilowattHours {
        Quantity(self.energy.actual_capacity().0 * self.settings.min_state_of_charge)
    }

    pub const fn max_residual_energy(&self) -> KilowattHours {
        Quantity(self.energy.actual_capacity().0 * self.settings.max_state_of_charge)
    }
}
