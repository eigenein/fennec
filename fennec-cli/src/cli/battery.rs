//! Battery-related CLI arguments.

use std::time::Duration;

use clap::Parser;
use tokio::time::timeout;
use tokio_modbus::{Slave, client::tcp::attach_slave};

use crate::{api::modbus, prelude::*, quantity::power::Kilowatts};

#[derive(Parser)]
pub struct BatteryConnectionArgs {
    #[clap(long = "battery-host", env = "BATTERY_HOST")]
    host: String,

    #[clap(long = "battery-port", env = "BATTERY_PORT", default_value = "502")]
    port: u16,

    #[clap(long = "battery-slave-id", default_value = "1", env = "BATTERY_SLAVE_ID")]
    slave_id: u8,
}

impl BatteryConnectionArgs {
    const TIMEOUT: Duration = Duration::from_secs(10);

    #[instrument(skip_all)]
    pub async fn connect(&self) -> Result<modbus::LegacyClient> {
        info!(
            host = self.host,
            port = self.port,
            slave_id = self.slave_id,
            "connecting to the battery…",
        );
        let tcp_stream =
            timeout(Self::TIMEOUT, tokio::net::TcpStream::connect((self.host.as_str(), self.port)))
                .await
                .context("timed out while connecting to the battery")?
                .context("failed to connect to the battery")?;
        Ok(modbus::LegacyClient::from(attach_slave(tcp_stream, Slave(self.slave_id))))
    }
}

#[derive(Copy, Clone, Parser)]
pub struct BatteryRegisters {
    #[clap(flatten)]
    pub energy: BatteryEnergyStateRegisters,

    #[clap(flatten)]
    pub setting: BatterySettingRegisters,
}

#[derive(Copy, Clone, Parser)]
pub struct BatteryEnergyStateRegisters {
    #[clap(long, default_value = "39424", env = "SOC_REGISTER")]
    pub state_of_charge: u16,

    #[clap(long, default_value = "37624", env = "SOH_REGISTER")]
    pub state_of_health: u16,

    #[clap(long, default_value = "37635", env = "DESIGN_CAPACITY_REGISTER")]
    pub design_capacity: u16,
}

#[derive(Copy, Clone, Parser)]
pub struct BatterySettingRegisters {
    #[clap(long, default_value = "46611", env = "MIN_SOC_ON_GRID_REGISTER")]
    pub min_state_of_charge_on_grid: u16,

    #[clap(long, default_value = "46610", env = "MAX_SOC_REGISTER")]
    pub max_state_of_charge: u16,
}

#[derive(Copy, Clone, Parser)]
pub struct BatteryPowerLimits {
    /// Charging power in kilowatts.
    #[clap(
        long = "charging-power-kilowatts",
        default_value = "1.2",
        env = "CHARGING_POWER_KILOWATTS"
    )]
    pub charging: Kilowatts,

    /// Discharging power in kilowatts.
    #[clap(
        long = "discharging-power-kilowatts",
        default_value = "0.8",
        env = "DISCHARGING_POWER_KILOWATTS"
    )]
    pub discharging: Kilowatts,
}

#[derive(Parser)]
pub struct BatteryArgs {
    #[clap(flatten)]
    pub power_limits: BatteryPowerLimits,

    #[clap(flatten)]
    pub connection: BatteryConnectionArgs,

    #[clap(flatten)]
    pub registers: BatteryRegisters,
}
