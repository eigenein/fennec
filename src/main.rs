mod api;
mod cli;
mod core;
mod prelude;
mod render;
mod units;

use chrono::{DurationRound, Local, TimeDelta, Timelike, Utc};
use clap::Parser;
use itertools::{EitherOrBoth, Itertools};
use logfire::config::{ConsoleOptions, SendToLogfire};
use tracing::level_filters::LevelFilter;

use crate::{
    api::{foxess, nextenergy, weerlive},
    cli::{Args, BurrowArgs, BurrowCommand, Command, HuntArgs},
    core::{cache::Cache, metrics::Metrics, optimizer::Optimizer, series::Series, solution::Step},
    prelude::*,
    render::{render_time_slot_sequence, try_render_steps},
    units::power::Kilowatts,
};

#[tokio::main]
async fn main() -> Result {
    let _logfire_guard = logfire::configure()
        .with_console(Some(
            ConsoleOptions::default()
                .with_min_log_level(Level::INFO) // doesn't seem to work
                .with_include_timestamps(false),
        ))
        .send_to_logfire(SendToLogfire::IfTokenPresent)
        .with_default_level_filter(LevelFilter::INFO)
        .finish()?
        .shutdown_guard();

    let args = Args::parse();
    let fox_ess = foxess::Api::try_new(args.fox_ess_api.api_key)?;

    match args.command {
        Command::Hunt(hunt_args) => {
            hunt(fox_ess, &args.fox_ess_api.serial_number, hunt_args).await?;
        }
        Command::Burrow(burrow_args) => {
            burrow(fox_ess, &args.fox_ess_api.serial_number, burrow_args).await?;
        }
    }

    info!("Done!");
    Ok(())
}

async fn hunt(fox_ess: foxess::Api, serial_number: &str, hunt_args: HuntArgs) -> Result {
    ensure!(
        hunt_args.consumption.stand_by >= Kilowatts::ZERO,
        "stand-by consumption must be non-negative",
    );

    let mut cache = Cache::read_from("cache.json")?;
    let now = Local::now();

    let metrics: Series<Metrics> = {
        let next_energy = nextenergy::Api::try_new()?;
        let mut hourly_rates = next_energy.get_hourly_rates(now).await?;
        let next_day = (now + TimeDelta::days(1)).duration_trunc(TimeDelta::days(1))?;
        hourly_rates.extend(next_energy.get_hourly_rates(next_day).await?.into_iter());
        info!("Fetched energy rates", len = hourly_rates.len());

        let solar_power_density = weerlive::Api::new(
            &hunt_args.solar.weerlive_api_key,
            &weerlive::Location::coordinates(hunt_args.solar.latitude, hunt_args.solar.longitude),
        )
        .get(now)
        .await?;
        info!("Fetched solar power forecast", len = solar_power_density.len());

        // FIXME: this should be implemented via `Series`:
        hourly_rates
            .into_iter()
            .zip_longest(solar_power_density.into_iter())
            .filter_map(|pair| match pair {
                EitherOrBoth::Both((lhs_time, grid_rate), (rhs_time, solar_power_density)) => {
                    assert_eq!(lhs_time, rhs_time);
                    Some((
                        lhs_time,
                        Metrics { grid_rate, solar_power_density: Some(solar_power_density) },
                    ))
                }
                EitherOrBoth::Left((time, grid_rate)) => {
                    Some((time, Metrics { grid_rate, solar_power_density: None }))
                }
                EitherOrBoth::Right(_) => None,
            })
            .collect()
    };

    let (residual_energy, total_capacity) = {
        (
            fox_ess.get_device_variables(serial_number).await?.residual_energy,
            fox_ess.get_device_details(serial_number).await?.total_capacity(),
        )
    };
    info!("Fetched battery details", residual_energy, total_capacity);

    let start_time = Utc::now();
    let initial_schedule =
        metrics.iter().map(|(time, _)| (time, cache.schedule[time.hour() as usize])).collect();
    let (n_mutations_succeeded, solution) = Optimizer::builder()
        .metrics(&metrics)
        .pv_surface_area(hunt_args.solar.pv_surface)
        .residual_energy(residual_energy)
        .capacity(total_capacity)
        .battery(hunt_args.battery)
        .consumption(hunt_args.consumption)
        .n_steps(hunt_args.n_optimization_steps)
        .build()
        .run(initial_schedule)?;
    let run_duration = Utc::now() - start_time;

    let profit = solution.profit();
    info!(
        "Optimized",
        run_duration = format!("{:.1}s", run_duration.as_seconds_f64()),
        n_mutations_succeeded = n_mutations_succeeded,
        net_loss = format!("¢{:.0}", solution.net_loss * 100.0),
        without_battery = format!("¢{:.0}", solution.net_loss_without_battery * 100.0),
        profit = format!("¢{:.0}", profit * 100.0),
    );
    println!("{}", try_render_steps(&metrics, &solution.steps)?);

    // Update the cache and avoid collisions with the same hours next day:
    for (time, step) in solution.steps.iter().take(cache.schedule.len()) {
        cache.schedule[time.hour() as usize] = step.working_mode;
    }

    let time_slot_sequence = foxess::TimeSlotSequence::from_schedule(
        solution.steps.map(|step: Step| step.working_mode),
        &hunt_args.battery,
    )?;
    println!("{}", render_time_slot_sequence(&time_slot_sequence));

    if !hunt_args.scout {
        fox_ess.set_schedule(serial_number, time_slot_sequence.as_ref()).await?;
    }

    cache.write_to("cache.json")?;
    Ok(())
}

async fn burrow(fox_ess: foxess::Api, serial_number: &str, args: BurrowArgs) -> Result {
    match args.command {
        BurrowCommand::DeviceDetails => {
            let details = fox_ess.get_device_details(serial_number).await?;
            info!("Gotcha", total_capacity = details.total_capacity());
        }

        BurrowCommand::DeviceVariables => {
            let variables = fox_ess.get_device_variables(serial_number).await?;
            info!("Gotcha", residual_energy = variables.residual_energy);
        }

        BurrowCommand::RawDeviceVariables => {
            let response = fox_ess.get_devices_variables_raw(&[serial_number]).await?;
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
            let schedule = fox_ess.get_schedule(serial_number).await?;
            info!("Gotcha", enabled = schedule.is_enabled);
            println!("{}", render_time_slot_sequence(&schedule.groups));
        }
    }
    Ok(())
}
