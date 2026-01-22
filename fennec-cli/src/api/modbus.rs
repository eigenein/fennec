use tokio_modbus::{
    Slave,
    client::{Reader, tcp::attach_slave},
};

use crate::{
    cli::{BatteryConnectionArgs, BatteryRegisters},
    prelude::*,
    quantity::energy::KilowattHours,
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
        registers: BatteryRegisters,
    ) -> Result<BatteryState> {
        info!("Reading the battery state…");
        let design_energy = KilowattHours::from(
            // Stored in decawatts:
            0.01 * f64::from(self.read_holding_register(registers.design_energy).await?),
        );
        let state_of_charge =
            0.01 * f64::from(self.read_holding_register(registers.state_of_charge).await?);
        let state_of_health =
            0.01 * f64::from(self.read_holding_register(registers.state_of_health).await?);
        Ok(BatteryState::new(design_energy, state_of_health, state_of_charge))
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
    pub capacity: KilowattHours,
    pub residual_energy: KilowattHours,
}

impl BatteryState {
    pub fn new(capacity: KilowattHours, state_of_health: f64, state_of_charge: f64) -> Self {
        Self { capacity, residual_energy: capacity * state_of_health * state_of_charge }
    }
}
