extern crate core;

mod cli;
mod foxess;
mod nextenergy;
mod optimizer;
mod prelude;
mod units;

use chrono::{Local, TimeDelta, Timelike};
use clap::Parser;
use logfire::config::{ConsoleOptions, SendToLogfire};
#[cfg(test)]
use tracing::level_filters::LevelFilter;

use crate::{
    cli::{Args, Command, FoxEssCommand},
    foxess::{FoxEseTimeSlotSequence, FoxEss},
    nextenergy::NextEnergy,
    optimizer::{optimise, working_mode::WorkingModeHourlySchedule},
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
            let starting_hour = now.hour();
            let next_energy = NextEnergy::try_new()?;
            let mut hourly_rates = next_energy.get_hourly_rates(now.date(), starting_hour).await?;
            hourly_rates
                .extend(next_energy.get_hourly_rates(now.date() + TimeDelta::days(1), 0).await?);
            info!("Fetched energy rates");

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
            let (profit, working_mode_sequence) = optimise(
                &hourly_rates,
                residual_energy,
                Kilowatts::from_watts_u32(args.battery.stand_by_power_watts),
                args.battery.min_soc_percent,
                total_capacity,
                args.battery.power,
            )?;
            info!("Optimized", profit = profit.to_string());

            let daily_schedule = WorkingModeHourlySchedule::<24>::from_working_modes(
                starting_hour,
                working_mode_sequence,
            );

            let time_slot_sequence = FoxEseTimeSlotSequence::from_schedule(
                daily_schedule,
                args.battery.power,
                args.battery.min_soc_percent,
            )?;

            if !args.stalk {
                fox_ess.set_schedule(&args.fox_ess_api.serial_number, &time_slot_sequence).await?;
            }

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
