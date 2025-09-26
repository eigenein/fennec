mod api;
mod cli;
mod core;
mod database;
mod prelude;
mod render;
mod units;

use chrono::Local;
use clap::Parser;
use logfire::config::{ConsoleOptions, SendToLogfire};
use tracing::level_filters::LevelFilter;

use crate::{
    api::{foxess, heartbeat, home_assistant, nextenergy},
    cli::{Args, BurrowArgs, BurrowCommand, Command, HuntArgs},
    core::{
        cache::Cache,
        series::Series,
        solver::Solver,
        working_mode::WorkingMode as CoreWorkingMode,
    },
    database::Database,
    prelude::*,
    render::{render_time_slot_sequence, try_render_steps},
    units::{energy::KilowattHours, power::Kilowatts},
};

#[tokio::main]
async fn main() -> Result {
    let _ = dotenvy::dotenv();

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
            let battery_args = hunt_args.battery;

            #[allow(clippy::literal_string_with_formatting_args)]
            if let Err(error) = hunt(&fox_ess, &args.fox_ess_api.serial_number, *hunt_args).await {
                error!("Hunting failed: {error:#}");

                // Setting the default self-use schedule:
                let time_slot = foxess::TimeSlot {
                    is_enabled: true,
                    start_time: foxess::StartTime::MIDNIGHT,
                    end_time: foxess::EndTime::MIDNIGHT,
                    max_soc: 100,
                    min_soc_on_grid: battery_args.min_soc_percent,
                    feed_soc: battery_args.min_soc_percent,
                    feed_power_watts: battery_args.max_feed_power_watts(),
                    working_mode: foxess::WorkingMode::SelfUse,
                };
                fox_ess.set_schedule(&args.fox_ess_api.serial_number, &[time_slot]).await?;
            }
        }

        Command::Burrow(burrow_args) => {
            burrow(&fox_ess, &args.fox_ess_api.serial_number, burrow_args).await?;
        }
    }

    info!("Done!");
    Ok(())
}

async fn hunt(fox_ess: &foxess::Api, serial_number: &str, hunt_args: HuntArgs) -> Result {
    let mut cache = Cache::read_from("cache.toml")?;
    let database = Database::try_new(&hunt_args.mongodb_url).await?;

    let home_assistant = home_assistant::Api::try_new(
        &hunt_args.home_assistant.access_token,
        hunt_args.home_assistant.total_energy_usage_url,
    )?;
    let total_energy_usage = home_assistant.get_total_energy_usage().await?;

    cache.total_usage.try_push(
        total_energy_usage.last_reported_at,
        KilowattHours::from(total_energy_usage.value),
    )?;
    database
        .log_total_energy_usage(
            total_energy_usage.last_reported_at,
            KilowattHours::from(total_energy_usage.value),
        )
        .await?;

    let now = Local::now();
    let grid_rates = nextenergy::Api::try_new()?.get_hourly_rates_48h(now).await?;
    ensure!(!grid_rates.is_empty());
    info!("Fetched energy rates", len = grid_rates.len());

    let residual_energy = fox_ess.get_device_variables(serial_number).await?.residual_energy;
    let total_capacity = fox_ess.get_device_details(serial_number).await?.total_capacity();
    info!("Fetched battery details", residual_energy, total_capacity);

    // Calculate the stand-by consumption:
    let stand_by_power = cache
        .total_usage
        .resample_hourly()
        .collect::<Series<KilowattHours>>()
        .differentiate()
        .collect::<Series<Kilowatts>>()
        .average_hourly();

    let solution = Solver::builder()
        .grid_rates(&grid_rates)
        .residual_energy(residual_energy)
        .capacity(total_capacity)
        .battery(hunt_args.battery)
        .consumption(hunt_args.consumption)
        .stand_by_power(stand_by_power)
        .solve();

    let profit = solution.summary.profit();
    info!(
        "Optimized",
        net_loss = format!("¢{:.0}", solution.summary.net_loss * 100.0),
        without_battery = format!("¢{:.0}", solution.summary.net_loss_without_battery * 100.0),
        profit = format!("¢{:.0}", profit * 100.0),
    );
    println!("{}", try_render_steps(&grid_rates, &solution.steps)?);

    let schedule: Series<CoreWorkingMode> =
        solution.steps.into_iter().map(|(time, step)| (time, step.working_mode)).collect();
    let time_slot_sequence =
        foxess::TimeSlotSequence::from_schedule(&schedule, &hunt_args.battery)?;
    println!("{}", render_time_slot_sequence(&time_slot_sequence));

    if !hunt_args.scout {
        fox_ess.set_schedule(serial_number, time_slot_sequence.as_ref()).await?;
    }

    if let Some(heartbeat_url) = hunt_args.heartbeat_url {
        heartbeat::send(heartbeat_url).await;
    }

    #[allow(clippy::literal_string_with_formatting_args)]
    if let Err(error) = cache.write_to("cache.toml") {
        warn!("Failed to save the cache: {error:#}");
    }

    Ok(())
}

async fn burrow(fox_ess: &foxess::Api, serial_number: &str, args: BurrowArgs) -> Result {
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
