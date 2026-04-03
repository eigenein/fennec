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
///
/// For the register numbers, see:
///
/// - <https://raw.githubusercontent.com/openhab/openhab-addons/refs/heads/main/bundles/org.openhab.binding.modbus.foxinverter/src/main/java/org/openhab/binding/modbus/foxinverter/internal/MQ2200InverterRegisters.java>
/// - <https://raw.githubusercontent.com/solakon-de/solakon-one-homeassistant/refs/heads/main/custom_components/solakon_one/const.py>
#[must_use]
pub struct MQ2200(tokio_modbus::client::Context);

impl MQ2200 {
    const TIMEOUT: Duration = Duration::from_secs(10);

    #[instrument(skip_all, fields(address = address))]
    pub async fn connect(address: &str) -> Result<Self> {
        info!("connecting…");
        let tcp_stream = timeout(Self::TIMEOUT, TcpStream::connect(address))
            .await
            .context("timed out while connecting to the battery")?
            .context("failed to connect to the battery")?;
        tcp_stream.set_nodelay(true)?;
        info!("connected");
        Ok(Self(attach_slave(tcp_stream, Slave(1))))
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
        self.read_u16(46609).await.map(Percentage)
    }

    async fn read_min_soc_on_grid(&mut self) -> Result<Percentage> {
        self.read_u16(46611).await.map(Percentage)
    }

    async fn read_max_soc(&mut self) -> Result<Percentage> {
        self.read_u16(46610).await.map(Percentage)
    }

    async fn read_design_capacity(&mut self) -> Result<DecawattHours> {
        self.read_u16(37635).await.map(DecawattHours)
    }

    async fn read_state_of_charge(&mut self) -> Result<Percentage> {
        self.read_u16(39424).await.map(Percentage)
    }

    async fn read_state_of_health(&mut self) -> Result<Percentage> {
        self.read_u16(37624).await.map(Percentage)
    }

    /// Read total external active power.
    ///
    /// Positive means discharging, negative means charging.
    async fn read_active_power(&mut self) -> Result<Watts> {
        self.read_i32(39134).await.map(Into::into)
    }

    /// Read current EPS output power.
    async fn read_eps_active_power(&mut self) -> Result<Watts> {
        self.read_i32(39216).await.map(Into::into)
    }

    async fn read_u16(&mut self, address: Address) -> Result<u16> {
        self.read_holding_registers(address, 1)
            .await?
            .into_iter()
            .next()
            .with_context(|| format!("register #{address} returned no data"))
    }

    async fn read_i32(&mut self, address: Address) -> Result<i32> {
        let words = self.read_holding_registers(address, 2).await?;
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
            self.0
                .read_holding_registers(address, count)
                .await
                .context("protocol or network error")?
                .context("Modbus server error")
        };
        // FIXME: reconnect on timeout.
        timeout(Self::TIMEOUT, read)
            .await
            .context("timed out while reading the registers")?
            .inspect(|words| debug!(?words, "read"))
    }
}
