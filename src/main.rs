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
    strategy::{Optimizer, WorkingModeHourlySchedule},
    units::power::Kilowatts,
    weerlive::{Location, Weerlive},
};

#[expect(clippy::too_many_lines)]
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

            let now = Local::now();
            let starting_hour = now.hour();

            let next_energy = NextEnergy::try_new()?;
            let mut hourly_rates =
                next_energy.get_hourly_rates(now.date_naive(), starting_hour).await?;
            hourly_rates.extend(
                next_energy.get_hourly_rates((now + TimeDelta::days(1)).date_naive(), 0).await?,
            );
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
                &hunt_args.solar.weerlive_api_key,
                &Location::coordinates(hunt_args.solar.latitude, hunt_args.solar.longitude),
            )
            .get(now)
            .await?
            .into_iter()
            .map(|power| Kilowatts(power.0 * hunt_args.solar.pv_surface_square_meters))
            .collect();

            hourly_rates.truncate(solar_power.len().min(24)); // FIXME: allow 24-hour increments.
            let solution = Optimizer::builder()
                .hourly_rates(&hourly_rates)
                .solar_power(&solar_power)
                .residual_energy(residual_energy)
                .capacity(total_capacity)
                .battery(&hunt_args.battery)
                .consumption(&hunt_args.consumption)
                .build()
                .run()?;

            for ((((hour, rate), working_mode), forecast), solar_power) in (starting_hour..)
                .zip(hourly_rates)
                .zip(&solution.working_mode_sequence)
                .zip(&solution.outcome.forecast)
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
                max_charge_rate = solution.strategy.max_charging_rate.map(
                    |max_charging_rate| format!("¢{:.0}", max_charging_rate * Decimal::ONE_HUNDRED)
                ),
                min_discharge_rate =
                    solution.strategy.min_discharging_rate.map(|min_discharging_rate| format!(
                        "¢{:.0}",
                        min_discharging_rate * Decimal::ONE_HUNDRED
                    )),
                net_profit = format!("€{:.2}", solution.outcome.net_profit),
                residual_energy_value = format!("€{:.2}", solution.outcome.residual_energy_value),
                total_profit = format!("€{:.2}", solution.outcome.total_profit()),
            );

            let schedule = WorkingModeHourlySchedule::<24>::from_working_modes(
                starting_hour,
                solution.working_mode_sequence,
            );

            let time_slot_sequence = FoxEseTimeSlotSequence::from_schedule(
                starting_hour as usize,
                &schedule,
                &hunt_args.battery,
            )?;

            if !hunt_args.scout {
                fox_ess_api
                    .set_schedule(&args.fox_ess_api.serial_number, &time_slot_sequence)
                    .await?;
            }
        }

        Command::Burrow(burrow_args) => match burrow_args.command {
            BurrowCommand::DeviceDetails => {
                let details =
                    fox_ess_api.get_device_details(&args.fox_ess_api.serial_number).await?;
                info!("Gotcha", total_capacity = details.total_capacity().to_string());
            }

            BurrowCommand::DeviceVariables => {
                let variables =
                    fox_ess_api.get_device_variables(&args.fox_ess_api.serial_number).await?;
                info!("Gotcha", residual_energy = variables.residual_energy.to_string());
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
            }

            BurrowCommand::Schedule => {
                let schedule = fox_ess_api.get_schedule(&args.fox_ess_api.serial_number).await?;
                info!("Gotcha", enabled = schedule.is_enabled);
                schedule.groups.trace();
            }
        },
    }

    info!("Done!");
    Ok(())
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
