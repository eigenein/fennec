use std::{net::IpAddr, sync::Arc, time::Duration};

use clap::Parser;

use crate::{
    api::{homewizard, mini_qube},
    battery::WorkingMode,
    energy,
    math::smoothing::HalfLife,
    prelude::*,
    quantity::{power::Watts, price::KilowattHourPrice, time::Hours},
};

#[derive(Parser)]
#[command(author, version, about, propagate_version = true)]
#[must_use]
pub struct Args {
    #[clap(long = "sentry-dsn", env = "SENTRY_DSN")]
    pub sentry_dsn: Option<String>,

    #[clap(flatten)]
    pub bind: BindArgs,

    #[clap(flatten)]
    pub connections: ConnectionArgs,

    #[clap(flatten)]
    pub engine: EngineArgs,
}

#[derive(Parser)]
pub struct EngineArgs {
    #[clap(flatten)]
    pub battery: BatteryArgs,

    #[clap(long, env = "INTERVAL", default_value = "5s", value_parser = humantime::parse_duration)]
    pub interval: Duration,

    #[clap(long, env = "ENERGY_PROVIDER")]
    pub energy_provider: energy::Provider,

    #[clap(flatten)]
    pub energy_profile: EnergyProfileArgs,

    /// Do not push schedule to the device, dry run.
    #[clap(long, alias = "scout", env = "DRY_RUN")]
    pub dry_run: bool,
}

#[derive(Parser)]
pub struct EnergyProfileArgs {
    /// Half-life for exponential moving average when learning the energy balance profile:
    /// - after τ: the energy profile is 50% adapted to the new routine;
    /// - after 2τ: 75% adapted;
    /// - after 3τ: 87.5% adapted.
    #[clap(
        long = "energy-balance-half-life",
        env = "ENERGY_BALANCE_HALF_LIFE",
        default_value = "7d",
        value_parser = |value: &str| value.parse::<humantime::Duration>().map(HalfLife::from),
    )]
    pub balance_half_life: HalfLife<Hours>,

    #[clap(
        long = "n-energy-balance-harmonics",
        env = "N_ENERGY_BALANCE_HARMONICS",
        default_value = "12"
    )]
    pub n_balance_harmonics: usize,

    /// Battery parameters are learned with exponential moving average.
    /// This factor multiplied by the battery capacity defines the half-life in the units of energy.
    /// The residual energy change is then used to calculate smoothing at each parameter update.
    #[clap(
        long = "battery-efficiency-half-life-factor",
        env = "BATTERY_EFFICIENCY_HALF_LIFE_FACTOR",
        default_value = "10"
    )]
    pub battery_efficiency_half_life_factor: f64,
}

/// Web UI binding arguments.
#[derive(Copy, Clone, Parser)]
pub struct BindArgs {
    /// Web UI binding address.
    #[clap(long = "bind-address", env = "BIND_ADDRESS", default_value = "::")]
    pub address: IpAddr,

    /// Web UI binding port.
    #[clap(long = "bind-port", env = "BIND_PORT", default_value = "80")]
    pub port: u16,
}

/// Battery power limits.
///
/// TODO: we could use `Watts<u16>` here.
#[must_use]
#[derive(Copy, Clone, Parser)]
pub struct BatteryPowerLimits {
    /// Charging power in watts.
    #[clap(
        name = "charging_power",
        long = "charging-power-watts",
        default_value = "1200",
        env = "CHARGING_POWER_WATTS"
    )]
    pub charging: Watts,

    /// Discharging power in watts.
    #[clap(
        name = "discharging_power",
        long = "discharging-power-watts",
        default_value = "800",
        env = "DISCHARGING_POWER_WATTS"
    )]
    pub discharging: Watts,

    /// Inverter power limit in watts – limits the summed grid and EPS output when discharging.
    #[clap(
        name = "max_inverter_power_watts",
        long = "max-inverter-power-watts",
        default_value = "1200",
        env = "MAX_INVERTER_POWER_WATTS"
    )]
    pub max_inverter_power: Watts,
}

impl BatteryPowerLimits {
    /// Calculate the effective power limits given the average EPS power.
    pub fn max_effective_flow(self, average_eps_power: Watts) -> energy::Flow<Watts> {
        energy::Flow {
            import: self.charging,

            // EPS power does not compete with the grid output, hence adding it on top.
            // The total discharging power, however, is limited by the maximum inverter output.
            export: (self.discharging + average_eps_power).min(self.max_inverter_power),
        }
    }
}

#[derive(Clone, Parser)]
pub struct BatteryArgs {
    #[clap(
        long = "battery-working-modes",
        env = "WORKING_MODES",
        value_delimiter = ',',
        num_args = 1..,
        default_value = "harness,compensate,charge,self-use",
    )]
    pub working_modes: Vec<WorkingMode>,

    #[clap(flatten)]
    pub power_limits: BatteryPowerLimits,

    /// Battery health costs lost to the cycling, in ¤/kWh.
    #[clap(
        long = "battery-degradation-cost",
        env = "BATTERY_DEGRADATION_COST",
        default_value = "0.01"
    )]
    pub degradation_cost: KilowattHourPrice,
}

#[derive(Parser)]
pub struct ConnectionArgs {
    /// P1 meter measurement URL.
    #[clap(long = "grid-measurement-url", env = "GRID_MEASUREMENT_URL")]
    pub grid_measurement_url: homewizard::Url,

    /// Battery Modbus address. Only Fox ESS MiniQube is supported.
    #[clap(long = "battery-address", env = "BATTERY_ADDRESS")]
    pub battery_address: String,
}

impl ConnectionArgs {
    pub fn connect(self) -> Result<Connections> {
        Ok(Connections {
            grid_measurement: self.grid_measurement_url.client()?,
            battery: Arc::new(mini_qube::Client::new(self.battery_address)),
        })
    }
}

#[derive(Clone)]
pub struct Connections {
    pub grid_measurement: homewizard::Client,
    pub battery: Arc<mini_qube::Client>,
}
