mod cli;
mod foxess;
mod nextenergy;
mod prelude;
mod strategy;
mod units;
mod weerlive;

use chrono::{Local, TimeDelta, Timelike};
use clap::Parser;
use logfire::config::{ConsoleOptions, SendToLogfire};
use rust_decimal::Decimal;
use tracing::level_filters::LevelFilter;

use crate::{
    cli::{Args, BurrowCommand, Command},
    foxess::{FoxEseTimeSlotSequence, FoxEssApi},
    nextenergy::NextEnergy,
    prelude::*,
    strategy::{Optimization, WorkingModeHourlySchedule},
    units::power::Kilowatts,
    weerlive::{Location, Weerlive},
};

#[allow(clippy::too_many_lines)]
#[tokio::main]
async fn main() -> Result {
    let _logfire_guard = logfire::configure()
        .with_console(Some(ConsoleOptions::default().with_include_timestamps(false)))
        .send_to_logfire(SendToLogfire::IfTokenPresent)
        .with_default_level_filter(LevelFilter::INFO)
        .finish()?
        .shutdown_guard();

    let args = Args::parse();
    let fox_ess_api = FoxEssApi::try_new(args.fox_ess_api.api_key)?;

    match args.command {
        Command::Hunt(hunt_args) => {
            ensure!(
                hunt_args.consumption.stand_by_power <= Kilowatts::ZERO,
                "stand-by consumption must be non-positive",
            );

            let now = Local::now().naive_local();
            let start_hour = now.hour();

            let next_energy = NextEnergy::try_new()?;
            let mut hourly_rates = next_energy.get_hourly_rates(now.date(), start_hour).await?;
            hourly_rates
                .extend(next_energy.get_hourly_rates(now.date() + TimeDelta::days(1), 0).await?);
            info!("Fetched energy rates", len = hourly_rates.len().to_string());

            let (residual_energy, total_capacity) = {
                (
                    fox_ess_api
                        .get_device_variables(&args.fox_ess_api.serial_number)
                        .await?
                        .residual_energy,
                    fox_ess_api
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

            let solar_power: Vec<_> = Weerlive::new(
                &hunt_args.pv.weerlive_api_key,
                &Location::coordinates(hunt_args.pv.latitude, hunt_args.pv.longitude),
            )
            .get(start_hour)
            .await?
            .into_iter()
            .map(|power| Kilowatts(power.0 * hunt_args.pv.pv_surface_square_meters))
            .collect();

            let optimization = Optimization::run(
                &hourly_rates,
                &solar_power,
                residual_energy,
                total_capacity,
                &hunt_args.battery,
                &hunt_args.consumption,
            )?;

            for ((((hour, rate), working_mode), forecast), solar_power) in (start_hour..)
                .zip(hourly_rates)
                .zip(&optimization.working_mode_sequence)
                .zip(&optimization.simulation.forecast)
                .zip(solar_power)
            {
                info!(
                    "Plan",
                    hour = (hour % 24).to_string(),
                    rate = format!("¢{:.0}", rate * Decimal::ONE_HUNDRED),
                    solar = format!("{:.2}㎾", solar_power),
                    before = format!("{:.2}", forecast.residual_energy_before),
                    mode = format!("{working_mode:?}"),
                    grid = format!("{:.2}", forecast.grid_energy_used),
                    after = format!("{:.2}", forecast.residual_energy_after),
                    profit = format!("¢{:.0}", forecast.net_profit * 100.0),
                );
            }
            info!(
                "Optimized",
                max_charge_rate =
                    format!("¢{:.0}", optimization.max_charge_rate * Decimal::ONE_HUNDRED),
                min_discharge_rate =
                    format!("¢{:.0}", optimization.min_discharge_rate * Decimal::ONE_HUNDRED),
                profit = format!("€{:.2}", optimization.simulation.net_profit),
            );

            let daily_schedule = WorkingModeHourlySchedule::<24>::from_working_modes(
                start_hour,
                optimization.working_mode_sequence,
            );

            let time_slot_sequence =
                FoxEseTimeSlotSequence::from_schedule(daily_schedule, &hunt_args.battery)?;

            if !hunt_args.scout {
                fox_ess_api
                    .set_schedule(&args.fox_ess_api.serial_number, &time_slot_sequence)
                    .await?;
            }

            Ok(())
        }

        Command::Burrow(burrow_args) => match burrow_args.command {
            BurrowCommand::DeviceDetails => {
                let details =
                    fox_ess_api.get_device_details(&args.fox_ess_api.serial_number).await?;
                info!("Gotcha", total_capacity = details.total_capacity().to_string());
                Ok(())
            }

            BurrowCommand::DeviceVariables => {
                let variables =
                    fox_ess_api.get_device_variables(&args.fox_ess_api.serial_number).await?;
                info!("Gotcha", residual_energy = variables.residual_energy.to_string());
                Ok(())
            }

            BurrowCommand::RawDeviceVariables => {
                let response = fox_ess_api
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

            BurrowCommand::Schedule => {
                let schedule = fox_ess_api.get_schedule(&args.fox_ess_api.serial_number).await?;
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
