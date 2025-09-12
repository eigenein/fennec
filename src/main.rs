mod api;
mod cli;
mod prelude;
mod strategy;
mod units;

use chrono::{Local, TimeDelta, Timelike, Utc};
use clap::Parser;
use logfire::config::{ConsoleOptions, SendToLogfire};
use tracing::level_filters::LevelFilter;

use crate::{
    api::{FoxEss, FoxEssTimeSlotSequence, NextEnergy, Weerlive, WeerliveLocation},
    cli::{Args, BurrowCommand, Command},
    prelude::*,
    strategy::{Optimizer, WorkingModeSchedule},
    units::Kilowatts,
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
    let foxess = FoxEss::try_new(args.fox_ess_api.api_key)?;

    match args.command {
        Command::Hunt(hunt_args) => {
            ensure!(
                hunt_args.consumption.stand_by >= Kilowatts::ZERO,
                "stand-by consumption must be non-negative",
            );

            let now = Local::now();
            let starting_hour = now.hour();

            let next_energy = NextEnergy::try_new()?;
            let mut hourly_rates =
                next_energy.get_hourly_rates(now.date_naive(), starting_hour).await?;
            hourly_rates.extend(
                next_energy.get_hourly_rates((now + TimeDelta::days(1)).date_naive(), 0).await?,
            );
            info!("Fetched energy rates", len = hourly_rates.len());

            let (residual_energy, total_capacity) = {
                (
                    foxess
                        .get_device_variables(&args.fox_ess_api.serial_number)
                        .await?
                        .residual_energy,
                    foxess
                        .get_device_details(&args.fox_ess_api.serial_number)
                        .await?
                        .total_capacity(),
                )
            };
            info!("Fetched battery details", residual_energy, total_capacity);

            let solar_power: Vec<_> = Weerlive::new(
                &hunt_args.solar.weerlive_api_key,
                &WeerliveLocation::coordinates(hunt_args.solar.latitude, hunt_args.solar.longitude),
            )
            .get(now)
            .await?
            .into_iter()
            .map(|power| Kilowatts::from(power.0 * hunt_args.solar.pv_surface_square_meters))
            .collect();
            info!("Fetched solar power forecast", len = solar_power.len());

            let start_time = Utc::now();
            let solution = Optimizer::builder()
                .hourly_rates(&hourly_rates)
                .solar_power(&solar_power)
                .residual_energy(residual_energy)
                .capacity(total_capacity)
                .battery(&hunt_args.battery)
                .consumption(&hunt_args.consumption)
                .n_steps(hunt_args.n_optimization_steps)
                .build()
                .run();
            let run_duration = Utc::now() - start_time;

            for (((hour, rate), step), solar_power) in
                (starting_hour..).zip(hourly_rates).zip(&solution.plan.steps).zip(solar_power)
            {
                info!(
                    "Plan",
                    hour = (hour % 24).to_string(),
                    rate = format!("¢{:.0}", rate * 100.0),
                    solar = format!("{:.2}㎾", solar_power),
                    before = format!("{:.2}", step.residual_energy_before),
                    mode = format!("{:?}", step.working_mode),
                    after = format!("{:.2}", step.residual_energy_after),
                    total = format!("{:.2}", step.total_consumption),
                    loss = format!("¢{:.0}", step.loss * 100.0),
                );
            }
            info!(
                "Optimized",
                run_duration = format!("{:.1}s", run_duration.as_seconds_f64()),
                net_loss = format!("¢{:.0}", solution.plan.net_loss * 100.0),
                without_battery = format!("¢{:.0}", solution.plan.net_loss_without_battery * 100.0),
                profit = format!("¢{:.0}", solution.plan.profit() * 100.0),
            );

            let schedule = WorkingModeSchedule::<24>::from_working_modes(
                starting_hour,
                solution.plan.steps.iter().map(|step| step.working_mode),
            );

            let time_slot_sequence = FoxEssTimeSlotSequence::from_schedule(
                starting_hour as usize,
                &schedule,
                &hunt_args.battery,
            )?;

            if !hunt_args.scout {
                foxess.set_schedule(&args.fox_ess_api.serial_number, &time_slot_sequence).await?;
            }
        }

        Command::Burrow(burrow_args) => match burrow_args.command {
            BurrowCommand::DeviceDetails => {
                let details = foxess.get_device_details(&args.fox_ess_api.serial_number).await?;
                info!("Gotcha", total_capacity = details.total_capacity());
            }

            BurrowCommand::DeviceVariables => {
                let variables =
                    foxess.get_device_variables(&args.fox_ess_api.serial_number).await?;
                info!("Gotcha", residual_energy = variables.residual_energy);
            }

            BurrowCommand::RawDeviceVariables => {
                let response = foxess
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
                let schedule = foxess.get_schedule(&args.fox_ess_api.serial_number).await?;
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
