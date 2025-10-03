#![allow(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]

mod api;
mod cli;
mod core;
mod prelude;
mod quantity;
mod render;

use chrono::{Local, TimeDelta};
use clap::Parser;
use logfire::config::{ConsoleOptions, SendToLogfire};
use tracing::level_filters::LevelFilter;

use crate::{
    api::{
        foxess,
        heartbeat,
        home_assistant,
        home_assistant::{battery::BatteryStateAttributes, history::State},
        nextenergy,
    },
    cli::{Args, BurrowCommand, BurrowFoxEssArgs, BurrowFoxEssCommand, Command, HuntArgs},
    core::{series::Series, solver::Solver, working_mode::WorkingMode as CoreWorkingMode},
    prelude::*,
    quantity::energy::KilowattHours,
    render::{render_time_slot_sequence, try_render_steps},
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
            hunt(&fox_ess, &args.fox_ess_api.serial_number, *hunt_args).await?;
        }
        Command::Burrow(burrow_args) => match burrow_args.command {
            BurrowCommand::FoxEss(burrow_args) => {
                burrow(&fox_ess, &args.fox_ess_api.serial_number, burrow_args).await?;
            }
        },
    }

    info!("Done!");
    Ok(())
}

async fn hunt(fox_ess: &foxess::Api, serial_number: &str, hunt_args: HuntArgs) -> Result {
    let home_assistant = home_assistant::Api::try_new(
        &hunt_args.home_assistant.access_token,
        hunt_args.home_assistant.base_url,
    )?;

    let now = Local::now();
    let grid_rates = nextenergy::Api::try_new()?.get_hourly_rates_48h(now).await?;
    ensure!(!grid_rates.is_empty());
    info!("Fetched energy rates", len = grid_rates.len());

    let residual_energy = fox_ess.get_device_variables(serial_number).await?.residual_energy;
    let total_capacity = fox_ess.get_device_details(serial_number).await?.total_capacity();
    info!("Fetched battery details", residual_energy, total_capacity);

    // Fetch the state history and resample it:
    let energy_differentials = home_assistant
        .get_history::<BatteryStateAttributes<KilowattHours>>(
            &hunt_args.home_assistant.total_energy_usage_entity_id,
            now - TimeDelta::days(hunt_args.home_assistant.n_history_days),
            now,
        )
        .await?
        .into_iter()
        .map(State::into)
        .collect::<Series<_>>()
        .resample_hourly()
        .collect::<Series<_>>()
        .differentiate()
        .collect::<Series<_>>();
    // Calculate the stand-by consumption:
    let stand_by_power = energy_differentials
        .into_iter()
        .map(|(timestamp, state)| (timestamp, state.total_energy_usage))
        .collect::<Series<_>>()
        .average_hourly();

    let solution = Solver::builder()
        .grid_rates(&grid_rates)
        .residual_energy(residual_energy)
        .capacity(total_capacity)
        .battery(hunt_args.battery)
        .consumption(hunt_args.consumption)
        .stand_by_power(stand_by_power)
        .now(now)
        .solve();

    let profit = solution.summary.profit();
    info!(
        "Optimized",
        net_loss = solution.summary.net_loss,
        without_battery = solution.summary.net_loss_without_battery,
        profit = profit,
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

    Ok(())
}

async fn burrow(fox_ess: &foxess::Api, serial_number: &str, args: BurrowFoxEssArgs) -> Result {
    match args.command {
        BurrowFoxEssCommand::DeviceDetails => {
            let details = fox_ess.get_device_details(serial_number).await?;
            info!("Gotcha", total_capacity = details.total_capacity());
        }

        BurrowFoxEssCommand::DeviceVariables => {
            let variables = fox_ess.get_device_variables(serial_number).await?;
            info!("Gotcha", residual_energy = variables.residual_energy);
        }

        BurrowFoxEssCommand::RawDeviceVariables => {
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

        BurrowFoxEssCommand::Schedule => {
            let schedule = fox_ess.get_schedule(serial_number).await?;
            info!("Gotcha", enabled = schedule.is_enabled);
            println!("{}", render_time_slot_sequence(&schedule.groups));
        }
    }
    Ok(())
}
