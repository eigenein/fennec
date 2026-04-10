//! FoxESS Modbus clients.

use std::time::Duration;

use tokio::{net::TcpStream, time::timeout};
use tokio_modbus::{
    Address,
    Slave,
    client::{Reader, tcp::attach_slave},
};

use crate::{
    battery,
    prelude::*,
    quantity::{energy::DecawattHours, power::Watts, ratios::Percentage},
};

/// FoxESS MQ2200 Modbus client.
#[must_use]
pub struct MQ2200 {
    address: String,
    context: Option<tokio_modbus::client::Context>,
}

impl MQ2200 {
    const TIMEOUT: Duration = Duration::from_secs(5);

    pub async fn connect(address: String) -> Result<Self> {
        let mut this = Self { address, context: None };
        this.get_context().await?;
        Ok(this)
    }

    #[instrument(skip_all, fields(address = self.address))]
    async fn get_context(&mut self) -> Result<&mut tokio_modbus::client::Context> {
        #[expect(clippy::unnecessary_unwrap)]
        if self.context.is_some() {
            return Ok(self.context.as_mut().unwrap());
        }

        info!("connecting…");
        let tcp_stream = timeout(Self::TIMEOUT, TcpStream::connect(&self.address))
            .await
            .context("timed out while connecting to the battery")?
            .context("failed to connect to the battery")?;
        tcp_stream.set_nodelay(true)?;
        info!("connected");
        Ok(self.context.insert(attach_slave(tcp_stream, Slave(1))))
    }

    #[instrument(skip_all)]
    pub async fn read_state(&mut self) -> Result<battery::State> {
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

    async fn read_min_system_soc(&mut self) -> Result<Percentage> {
        self.read_u16(46609).await.context("failed to read the minimum system SoC").map(Percentage)
    }

    async fn read_min_soc_on_grid(&mut self) -> Result<Percentage> {
        self.read_u16(46611).await.context("failed to read the minimum SoC on grid").map(Percentage)
    }

    async fn read_max_soc(&mut self) -> Result<Percentage> {
        self.read_u16(46610).await.context("failed to read the maximum SoC").map(Percentage)
    }

    async fn read_design_capacity(&mut self) -> Result<DecawattHours> {
        self.read_u16(37635).await.context("failed to read the design capacity").map(DecawattHours)
    }

    async fn read_state_of_charge(&mut self) -> Result<Percentage> {
        self.read_u16(39424).await.context("failed to read the SoC").map(Percentage)
    }

    async fn read_state_of_health(&mut self) -> Result<Percentage> {
        self.read_u16(37624).await.context("failed to read the SoH").map(Percentage)
    }

    /// Read total external active power.
    ///
    /// Positive means discharging, negative means charging.
    async fn read_active_power(&mut self) -> Result<Watts> {
        self.read_i32(39134).await.context("failed to read the active power").map(Into::into)
    }

    /// Read current EPS output power.
    async fn read_eps_active_power(&mut self) -> Result<Watts> {
        self.read_i32(39216).await.context("failed to read the EPS active power").map(Into::into)
    }

    async fn read_u16(&mut self, address: Address) -> Result<u16> {
        self.read_holding_registers(address, 1)
            .await
            .context("failed to read `u16`")?
            .into_iter()
            .next()
            .with_context(|| format!("register #{address} returned no data"))
    }

    async fn read_i32(&mut self, address: Address) -> Result<i32> {
        let words =
            self.read_holding_registers(address, 2).await.context("failed to read `u32`")?;
        let [high, low] = words[..] else {
            bail!("register #{address} returned {} words, expected 2", words.len());
        };
        let [high_1, high_0] = high.to_be_bytes();
        let [low_1, low_0] = low.to_be_bytes();
        Ok(i32::from_be_bytes([high_1, high_0, low_1, low_0]))
    }

    #[instrument(skip_all, fields(address = address))]
    async fn read_holding_registers(&mut self, address: Address, count: u16) -> Result<Vec<u16>> {
        let read = async {
            self.get_context()
                .await?
                .read_holding_registers(address, count)
                .await
                .context("Modbus protocol or network error")?
                .context("Modbus server error")
        };
        timeout(Self::TIMEOUT, read)
            .await
            .map_err(Error::from)
            .flatten()
            .inspect(|words| debug!(?words, "read"))
            .inspect_err(|_| {
                self.context = None;
            })
    }
}
