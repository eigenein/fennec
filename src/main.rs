mod api;
mod cli;
mod core;
mod prelude;
mod render;
mod units;

use chrono::{Local, Utc};
use clap::Parser;
use logfire::config::{ConsoleOptions, SendToLogfire};
use tracing::level_filters::LevelFilter;

use crate::{
    api::{foxess, heartbeat, home_assistant, nextenergy, weerlive},
    cli::{Args, BurrowArgs, BurrowCommand, Command, HuntArgs},
    core::{
        cache::Cache,
        metrics::Metrics,
        optimizer::Optimizer,
        series::Series,
        working_mode::WorkingMode,
    },
    prelude::*,
    render::{render_time_slot_sequence, try_render_steps},
    units::{energy::KilowattHours, power::Kilowatts},
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
            hunt(fox_ess, &args.fox_ess_api.serial_number, *hunt_args).await?;
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

    if let Some((ha_token, ha_url)) = hunt_args.home_assistant.into_tuple() {
        let total_energy_usage =
            home_assistant::Api::try_new(&ha_token, ha_url)?.get_total_energy_usage().await?;
        cache.total_usage.insert(
            total_energy_usage.last_reported_at,
            KilowattHours::from(total_energy_usage.value),
        );
    }

    let metrics: Series<Metrics> = {
        let now = Local::now();

        let grid_rates = nextenergy::Api::try_new()?.get_hourly_rates_48h(now).await?;
        info!("Fetched energy rates", len = grid_rates.len());

        let solar_power_density = weerlive::Api::new(
            &hunt_args.solar.weerlive_api_key,
            &weerlive::Location::coordinates(hunt_args.solar.latitude, hunt_args.solar.longitude),
        )
        .get(now)
        .await?;
        info!("Fetched solar power forecast", len = solar_power_density.len());

        grid_rates
            .zip_right_or(&solar_power_density, |power_density| Some(*power_density), None)
            .map(|(time, (grid_rate, solar_power_density))| {
                (*time, Metrics { grid_rate: *grid_rate, solar_power_density })
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

    // Build the initial schedule from the cached one: drop the old entries and fill the future
    // entries in with the default mode:
    let initial_schedule: Series<WorkingMode> = metrics
        .zip_right_or(&cache.solution, |mode| *mode, WorkingMode::default())
        .map(|(time, (_, mode))| (*time, mode))
        .collect();

    let start_time = Utc::now();
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

    let schedule: Series<WorkingMode> =
        solution.steps.into_iter().map(|(time, step)| (time, step.working_mode)).collect();

    let time_slot_sequence =
        foxess::TimeSlotSequence::from_schedule(&schedule, &hunt_args.battery)?;
    println!("{}", render_time_slot_sequence(&time_slot_sequence));

    // Update the cache:
    cache.solution = schedule;

    if !hunt_args.scout {
        fox_ess.set_schedule(serial_number, time_slot_sequence.as_ref()).await?;
    }

    if let Some(heartbeat_url) = hunt_args.heartbeat_url {
        heartbeat::send(heartbeat_url).await;
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
