pub mod hunter;
pub mod logger;
mod sentry;

use std::{net::IpAddr, sync::Arc, time::Duration};

use clap::Parser;
use tokio::{spawn, sync::RwLock, try_join};

use crate::{
    api::{battery, homewizard},
    battery::WorkingMode,
    cli::sentry::SentryArgs,
    cron::CronSchedule,
    energy,
    math::smoothing::HalfLife,
    prelude::*,
    quantity::{power::Watts, price::KilowattHourPrice},
    web,
};

#[derive(Parser)]
#[command(author, version, about, propagate_version = true)]
#[must_use]
pub struct Args {
    #[clap(flatten)]
    pub sentry: SentryArgs,

    #[clap(flatten)]
    connections: ConnectionArgs,

    #[clap(flatten)]
    bind: BindArgs,

    #[clap(flatten)]
    battery: BatteryArgs,

    #[clap(long, env = "LOGGER_CRON", default_value = "*/5 * * * * *")]
    logger_cron: CronSchedule,

    #[clap(long, env = "OPTIMIZER_CRON", default_value = "0 */15 * * * *")]
    optimizer_cron: CronSchedule,

    #[clap(long, env = "ENERGY_PROVIDER")]
    energy_provider: energy::Provider,

    /// Half-life for exponential moving average when learning the energy balance:
    /// - after τ: the energy profile is 50% adapted to the new routine;
    /// - after 2τ: 75% adapted;
    /// - after 3τ: 87.5% adapted.
    #[clap(long, env = "ENERGY_BALANCE_HALF_LIFE", default_value = "7d")]
    energy_balance_half_life: humantime::Duration,

    #[clap(long, env = "N_BALANCE_HARMONICS", default_value = "12")]
    n_balance_harmonics: usize,

    /// Battery parameters are learned with exponential moving average.
    /// This factor multiplied by the battery capacity defines the half-life in the units of energy.
    /// The residual energy change is then used to calculate smoothing at each parameter update.
    #[clap(long, env = "BATTERY_EFFICIENCY_HALF_LIFE_FACTOR", default_value = "10")]
    battery_efficiency_half_life_factor: f64,

    /// Do not push schedule to the device, dry run.
    #[clap(long, alias = "scout", env = "DRY_RUN")]
    dry_run: bool,
}

impl Args {
    pub async fn run(self) -> Result {
        let battery_power_limits = self.battery.power_limits;
        let connections = self.connections.connect()?;

        let logger_runner = logger::Args::builder()
            .connections(connections.clone())
            .battery_power_limits(battery_power_limits)
            .energy_balance_half_life(HalfLife(
                Duration::from(self.energy_balance_half_life).into(),
            ))
            .battery_efficiency_half_life_factor(self.battery_efficiency_half_life_factor)
            .n_balance_harmonics(self.n_balance_harmonics)
            .build()
            .start()
            .await?;
        let hunter_runner = hunter::Runner::builder()
            .connections(connections.clone())
            .energy_provider(self.energy_provider)
            .battery_args(self.battery)
            .n_balance_harmonics(self.n_balance_harmonics)
            .dry_run(self.dry_run)
            .build();

        let hunter_state = Arc::new(RwLock::new(hunter_runner.run_once().await?));
        let state =
            web::State { hunter: hunter_state.clone(), logger_runner: logger_runner.clone() };
        try_join!(
            async { spawn(logger_runner.run_forever(self.logger_cron)).await? },
            async { spawn(hunter_runner.run_forever(self.optimizer_cron, hunter_state)).await? },
            async { spawn(web::serve(self.bind.address, self.bind.port, state)).await? },
        )?;

        Ok(())
    }
}

/// Web UI binding arguments.
#[derive(Parser)]
pub struct BindArgs {
    /// Web UI binding address.
    #[clap(long = "bind-address", env = "BIND_ADDRESS", default_value = "::")]
    pub address: IpAddr,

    /// Web UI binding port.
    #[clap(long = "bind-port", env = "BIND_PORT", default_value = "80")]
    pub port: u16,
}

/// Battery power limits.
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

#[derive(Parser)]
pub struct BatteryArgs {
    #[clap(
        long,
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
    #[clap(long, env = "GRID_MEASUREMENT_URL")]
    grid_measurement_url: homewizard::Url,

    /// Battery Modbus address. Currently, only FoxESS MQ2200 is supported.
    #[clap(long = "battery-address", env = "BATTERY_ADDRESS")]
    battery_address: String,
}

impl ConnectionArgs {
    /// TODO: inline when hunter and logger would be combined.
    pub fn connect(self) -> Result<Connections> {
        Ok(Connections {
            grid_measurement: self.grid_measurement_url.client()?,
            battery: Arc::new(battery::Client::new(self.battery_address)),
        })
    }
}

#[derive(Clone)]
pub struct Connections {
    pub grid_measurement: homewizard::Client,
    pub battery: Arc<battery::Client>,
}
