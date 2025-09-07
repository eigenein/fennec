extern crate core;

mod cli;
mod foxess;
mod nextenergy;
mod optimizer;
mod prelude;
mod units;

use chrono::Local;
use clap::Parser;
use logfire::config::{ConsoleOptions, SendToLogfire};
#[cfg(test)]
use tracing::level_filters::LevelFilter;

use crate::{
    cli::{Args, Command, FoxEssCommand},
    foxess::{FoxEseTimeSlotSequence, FoxEss},
    nextenergy::NextEnergy,
    optimizer::optimise,
    prelude::*,
    units::Kilowatts,
};

#[allow(clippy::too_many_lines)]
#[tokio::main]
async fn main() -> Result {
    let _logfire_guard = logfire::configure()
        .with_console(Some(ConsoleOptions::default().with_include_timestamps(false)))
        .send_to_logfire(SendToLogfire::IfTokenPresent)
        .finish()?
        .shutdown_guard();

    match Args::parse().command {
        Command::Hunt(args) => {
            let now = Local::now().naive_local();
            let hourly_rates = NextEnergy::try_new()?.get_upcoming_hourly_rates(now).await?;
            info!("Fetched energy rates", n_rates = hourly_rates.len().to_string());

            let fox_ess = FoxEss::try_new(args.fox_ess_api.api_key)?;
            let (residual_energy, total_capacity) = {
                (
                    fox_ess
                        .get_device_variables(&args.fox_ess_api.serial_number)
                        .await?
                        .residual_energy,
                    fox_ess
                        .get_device_details(&args.fox_ess_api.serial_number)
                        .await?
                        .total_capacity(),
                )
            };
            info!(
                "Fetched battery details",
                residual_energy = residual_energy.to_string(),
                total_capacity = total_capacity.to_string(),
            );
            let (profit, battery_plan) = optimise(
                &hourly_rates,
                residual_energy,
                Kilowatts::from_watts_u32(args.battery.stand_by_power_watts),
                args.battery.min_soc_percent,
                total_capacity,
                args.battery.power,
            )?;
            info!("Optimized", profit = profit.to_string());
            battery_plan.trace();

            let schedule_groups = FoxEseTimeSlotSequence::from_battery_plan(
                now,
                battery_plan,
                args.battery.power,
                args.battery.min_soc_percent,
            );
            info!("Compiled schedule", n_groups = schedule_groups.0.len().to_string());
            schedule_groups.trace();

            if !args.stalk {
                fox_ess.set_schedule(&args.fox_ess_api.serial_number, &schedule_groups).await?;
            }

            Ok(())
        }

        Command::Scout(args) => {
            let hourly_rates = NextEnergy::try_new()?
                .get_upcoming_hourly_rates(Local::now().naive_local())
                .await?;
            for rate in &hourly_rates {
                info!(
                    "Rate",
                    start_time = rate.start_at.to_string(),
                    value = rate.energy_rate.to_string(),
                );
            }

            let (profit, schedule) = optimise(
                &hourly_rates,
                args.residual_energy,
                Kilowatts::from_watts_u32(args.battery.stand_by_power_watts),
                args.battery.min_soc_percent,
                args.capacity,
                args.battery.power,
            )?;
            schedule.trace();
            info!("Final", profit = profit.to_string());

            Ok(())
        }

        Command::DebugFoxEss(args) => match args.command {
            FoxEssCommand::DeviceDetails => {
                let details = FoxEss::try_new(args.fox_ess_api.api_key)?
                    .get_device_details(&args.fox_ess_api.serial_number)
                    .await?;
                info!("Gotcha", total_capacity = details.total_capacity().to_string());
                Ok(())
            }

            FoxEssCommand::DeviceVariables => {
                let variables = FoxEss::try_new(args.fox_ess_api.api_key)?
                    .get_device_variables(&args.fox_ess_api.serial_number)
                    .await?;
                info!("Gotcha", residual_energy = variables.residual_energy.to_string());
                Ok(())
            }

            FoxEssCommand::RawDeviceVariables => {
                let response = FoxEss::try_new(args.fox_ess_api.api_key)?
                    .get_devices_variables_raw(&[args.fox_ess_api.serial_number.as_str()])
                    .await?;
                info!("Gotcha!");
                for device in response {
                    for variable in device.variables {
                        info!(
                            "Variable",
                            serial_number = &device.serial_number,
                            name = variable.name,
                            description = variable.description,
                            unit = variable.unit,
                            value = variable.value.to_string(),
                        );
                    }
                }
                Ok(())
            }

            FoxEssCommand::Schedule => {
                let schedule = FoxEss::try_new(args.fox_ess_api.api_key)?
                    .get_schedule(&args.fox_ess_api.serial_number)
                    .await?;
                info!("Gotcha", enabled = schedule.is_enabled);
                schedule.groups.trace();
                Ok(())
            }
        },
    }
}

/// Configure Logfire for unit tests.
#[cfg(test)]
#[ctor::ctor]
fn init() {
    logfire::configure()
        .with_console(Some(
            ConsoleOptions::default()
                .with_include_timestamps(false)
                .with_min_log_level(Level::TRACE),
        ))
        .send_to_logfire(SendToLogfire::No)
        .with_default_level_filter(LevelFilter::TRACE)
        .finish()
        .unwrap();
}
