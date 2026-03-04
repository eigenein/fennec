//! FoxESS Modbus clients.

use std::time::Duration;

use tokio::{net::TcpStream, time::timeout};
use tokio_modbus::{
    Address,
    Slave,
    client::{Reader, tcp::attach_slave},
};

use crate::{
    core::battery,
    prelude::*,
    quantity::{energy::DecawattHours, ratios::Percentage},
};

/// [MQ2200]
#[must_use]
pub struct MQ2200(tokio_modbus::client::Context);

impl MQ2200 {
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

    #[instrument(skip_all, fields(address = address))]
    pub async fn connect(address: &str) -> Result<Self> {
        info!("connecting…");
        let tcp_stream = timeout(Self::CONNECT_TIMEOUT, TcpStream::connect(address))
            .await
            .context("timed out while connecting to the battery")?
            .context("failed to connect to the battery")?;
        tcp_stream.set_nodelay(true)?;
        info!("connected");
        Ok(Self(attach_slave(tcp_stream, Slave(1))))
    }

    pub async fn read_energy_state(&mut self) -> Result<battery::EnergyState> {
        Ok(battery::EnergyState {
            design_capacity: self.read_design_capacity().await?,
            state_of_charge: self.read_state_of_charge().await?,
            state_of_health: self.read_state_of_health().await?,
        })
    }

    pub async fn read_full_state(&mut self) -> Result<battery::FullState> {
        let min_state_of_charge = self.read_min_state_of_charge().await?;
        let max_state_of_charge = self.read_max_state_of_charge().await?;
        Ok(battery::FullState {
            energy: self.read_energy_state().await?,
            allowed_state_of_charge: (min_state_of_charge..=max_state_of_charge).into(),
        })
    }

    async fn read_min_state_of_charge(&mut self) -> Result<Percentage> {
        self.read_holding_register(46611).await.map(Percentage)
    }

    async fn read_max_state_of_charge(&mut self) -> Result<Percentage> {
        self.read_holding_register(46610).await.map(Percentage)
    }

    async fn read_design_capacity(&mut self) -> Result<DecawattHours> {
        self.read_holding_register(37635).await.map(DecawattHours)
    }

    async fn read_state_of_charge(&mut self) -> Result<Percentage> {
        self.read_holding_register(39424).await.map(Percentage)
    }

    async fn read_state_of_health(&mut self) -> Result<Percentage> {
        self.read_holding_register(37624).await.map(Percentage)
    }

    #[instrument(skip_all, fields(address = address))]
    async fn read_holding_register(&mut self, address: Address) -> Result<u16> {
        self.0
            .read_holding_registers(address, 1)
            .await??
            .into_iter()
            .next()
            .with_context(|| format!("register #{address} returned no data"))
            .inspect(|word| debug!(word, "read"))
    }
}
