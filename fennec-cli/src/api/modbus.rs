use tokio_modbus::{
    Slave,
    client::{Reader, tcp::attach_slave},
};

use crate::{
    cli::{BatteryConnectionArgs, BatteryStateRegisters},
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
    pub async fn read_battery_state(
        &mut self,
        registers: BatteryStateRegisters,
    ) -> Result<BatteryState> {
        info!("Reading the battery state…");
        let design_capacity = KilowattHours::from(
            // Stored in decawatts:
            0.01 * f64::from(self.read_holding_register(registers.design_capacity).await?),
        );
        let state_of_charge =
            0.01 * f64::from(self.read_holding_register(registers.state_of_charge).await?);
        let state_of_health =
            0.01 * f64::from(self.read_holding_register(registers.state_of_health).await?);
        Ok(BatteryState { design_capacity, state_of_charge, state_of_health })
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
pub struct BatteryState {
    design_capacity: KilowattHours,
    state_of_charge: f64,
    state_of_health: f64,
}

impl BatteryState {
    /// Battery capacity corrected on the state of health.
    pub const fn actual_capacity(&self) -> KilowattHours {
        Quantity(self.design_capacity.0 * self.state_of_health)
    }

    /// Residual energy corrected on the state of health.
    pub const fn residual_energy(&self) -> KilowattHours {
        Quantity(self.actual_capacity().0 * self.state_of_charge)
    }
}
