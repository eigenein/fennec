mod battery_state;

use std::time::Duration;

use derive_more::From;
use tokio::{net::TcpStream, time::timeout};
use tokio_modbus::{
    Slave,
    client::{Reader, tcp::attach_slave},
};
use url::{Host, Url};

use crate::{
    api::modbus::battery_state::{BatteryEnergyState, BatterySettings, BatteryState},
    cli::battery::{BatteryEnergyStateRegisters, BatteryRegisters, BatterySettingRegisters},
    ops::RangeInclusive,
    prelude::*,
};

#[must_use]
#[derive(From)]
pub struct Client(tokio_modbus::client::Context);

impl Client {
    const TIMEOUT: Duration = Duration::from_secs(10);

    #[instrument]
    pub async fn connect(url: Url) -> Result<Self> {
        info!("connecting…");
        if url.scheme() != "modbus+tcp://" {
            bail!("only `modbus+tcp://` is currently supported");
        }
        let host = url.host().context("the URL must contain host")?;
        let port = url.port().unwrap_or(502);
        let slave_id = url
            .fragment()
            .context("slave ID is not specified")?
            .parse()
            .context("incorrect slave ID")?;
        let tcp_stream = {
            let result = match host {
                Host::Domain(domain) => {
                    timeout(Self::TIMEOUT, TcpStream::connect((domain, port))).await
                }
                Host::Ipv4(ip_address) => {
                    timeout(Self::TIMEOUT, TcpStream::connect((ip_address, port))).await
                }
                Host::Ipv6(ip_address) => {
                    timeout(Self::TIMEOUT, TcpStream::connect((ip_address, port))).await
                }
            };
            result
                .context("timed out while connecting to the battery")?
                .context("failed to connect to the battery")?
        };
        tcp_stream.set_nodelay(true)?;
        Ok(Self(attach_slave(tcp_stream, Slave(slave_id))))
    }

    #[instrument(skip_all)]
    pub async fn read_energy_state(
        &mut self,
        registers: BatteryEnergyStateRegisters,
    ) -> Result<BatteryEnergyState> {
        let design_capacity = self.read_holding_register(registers.design_capacity).await?.into();
        let state_of_charge = self.read_holding_register(registers.state_of_charge).await?.into();
        let state_of_health = self.read_holding_register(registers.state_of_health).await?.into();
        info!(?state_of_charge, ?state_of_health, ?design_capacity, "fetched the battery state");
        Ok(BatteryEnergyState { design_capacity, state_of_charge, state_of_health })
    }

    #[instrument(skip_all)]
    pub async fn read_battery_settings(
        &mut self,
        registers: BatterySettingRegisters,
    ) -> Result<BatterySettings> {
        let min_state_of_charge =
            self.read_holding_register(registers.min_state_of_charge_on_grid).await?.into();
        let max_state_of_charge =
            self.read_holding_register(registers.max_state_of_charge).await?.into();
        info!(?min_state_of_charge, ?max_state_of_charge, "fetched the battery settings");
        Ok(BatterySettings {
            allowed_state_of_charge: RangeInclusive::from_std(
                min_state_of_charge..=max_state_of_charge,
            ),
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

    #[instrument(skip_all, level = "debug", fields(register = register))]
    async fn read_holding_register(&mut self, register: u16) -> Result<u16> {
        let value = self
            .0
            .read_holding_registers(register, 1)
            .await??
            .pop()
            .with_context(|| format!("nothing is read from the register #{register}"))?;
        Ok(value)
    }
}
