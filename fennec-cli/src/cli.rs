use std::{net::IpAddr, sync::Arc, time::Duration};

use crate::{
    api::{Connections, heartbeat, home_assistant, homewizard, mini_qube},
    battery,
    energy,
    math::smoothing::HalfLife,
    prelude::*,
    quantity::{energy::WattHours, time::Hours},
};

/// Root CLI arguments.
#[derive(clap::Parser)]
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

#[derive(clap::Args)]
pub struct EngineArgs {
    #[clap(flatten)]
    pub battery: battery::Args,

    #[clap(long, env = "INTERVAL", default_value = "5s", value_parser = humantime::parse_duration)]
    pub interval: Duration,

    #[clap(long, env = "ENERGY_PROVIDER")]
    pub energy_provider: energy::Provider,

    #[clap(flatten)]
    pub energy_profile: EnergyProfileArgs,

    #[clap(
        long = "min-final-residual-energy-watt-hours",
        env = "MIN_FINAL_RESIDUAL_ENERGY_WATT_HOURS",
        default_value = "0"
    )]
    pub min_final_residual_energy: WattHours<usize>,

    /// Do not push schedule to the device, dry run.
    #[clap(long, alias = "scout", env = "DRY_RUN")]
    pub dry_run: bool,
}

#[derive(clap::Args)]
pub struct EnergyProfileArgs {
    /// Half-life for exponential moving average when learning the energy balance profile:
    /// - after τ: the energy profile is 50% adapted to the new routine;
    /// - after 2τ: 75% adapted;
    /// - after 3τ: 87.5% adapted.
    #[clap(
        long = "energy-balance-half-life",
        env = "ENERGY_BALANCE_HALF_LIFE",
        default_value = "14d",
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
#[derive(Copy, Clone, clap::Args)]
pub struct BindArgs {
    /// Web UI binding address.
    #[clap(long = "bind-address", env = "BIND_ADDRESS", default_value = "::")]
    pub address: IpAddr,

    /// Web UI binding port.
    #[clap(long = "bind-port", env = "BIND_PORT", default_value = "80")]
    pub port: u16,
}

#[derive(clap::Args)]
pub struct ConnectionArgs {
    /// P1 meter measurement URL.
    #[clap(long = "grid-measurement-url", env = "GRID_MEASUREMENT_URL")]
    pub grid_measurement_url: homewizard::Url,

    /// Battery Modbus address. Only Fox ESS MiniQube is supported.
    #[clap(long = "battery-address", env = "BATTERY_ADDRESS")]
    pub battery_address: String,

    /// Heartbeat URL.
    #[clap(long = "heartbeat-url", env = "HEARTBEAT_URL")]
    pub heartbeat_url: Option<reqwest::Url>,

    /// Home Assistant REST API entity state URL.
    ///
    /// The URL must have the fragment set to the bearer token.
    /// Example: `https://homeassistant.local/api/states/sensor.custom_fennec_working_mode#0123...6789`.
    #[clap(long, env = "HOME_ASSISTANT_WORKING_MODE_URL")]
    pub home_assistant_working_mode_url: Option<reqwest::Url>,
}

impl ConnectionArgs {
    pub fn connect(self) -> Result<Connections> {
        Ok(Connections {
            grid_measurement: self.grid_measurement_url.client()?,
            battery: Arc::new(mini_qube::Client::new(self.battery_address)),
            heartbeat: heartbeat::Client::new(self.heartbeat_url)?,
            home_assistant_working_mode: home_assistant::StateClient::new(
                self.home_assistant_working_mode_url,
            )?,
        })
    }
}
