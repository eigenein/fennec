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
use itertools::Itertools;
use logfire::config::{ConsoleOptions, SendToLogfire};
use tracing::level_filters::LevelFilter;

use crate::{
    api::{foxess, heartbeat, nextenergy},
    cli::{Args, BurrowCommand, BurrowFoxEssArgs, BurrowFoxEssCommand, Command, HuntArgs},
    core::{series::Series, solver::Solver},
    prelude::*,
    quantity::{energy::KilowattHours, power::Kilowatts},
    render::{render_steps, render_time_slot_sequence},
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
                burrow_fox_ess(&fox_ess, &args.fox_ess_api.serial_number, burrow_args).await?;
            }
        },
    }

    info!("Done!");
    Ok(())
}

async fn hunt(fox_ess: &foxess::Api, serial_number: &str, hunt_args: HuntArgs) -> Result {
    let working_modes = hunt_args.working_modes();
    let home_assistant = hunt_args.home_assistant.connection.try_new_client()?;
    let now = Local::now();
    let history_period = (now - TimeDelta::days(hunt_args.home_assistant.n_history_days))..=now;

    let grid_rates: Series<_, _> =
        nextenergy::Api::try_new()?.get_hourly_rates_48h(now).await?.collect();
    ensure!(!grid_rates.is_empty());
    info!("Fetched energy rates", len = grid_rates.len());

    let residual_energy = fox_ess.get_device_variables(serial_number).await?.residual_energy;
    let total_capacity = fox_ess.get_device_details(serial_number).await?.total_capacity();
    info!("Fetched battery details", residual_energy, total_capacity);

    let stand_by_usage = home_assistant
        .get_average_hourly_deltas::<KilowattHours>(
            &hunt_args.home_assistant.total_usage_entity_id,
            &history_period,
        )
        .await?;
    let solar_yield = home_assistant
        .get_average_hourly_deltas::<KilowattHours>(
            &hunt_args.home_assistant.solar_yield_entity_id,
            &history_period,
        )
        .await?;

    // Calculate the stand-by power:
    let stand_by_power = stand_by_usage
        .into_iter()
        .zip(solar_yield)
        .map(|(usage, r#yield)| {
            usage.unwrap_or(Kilowatts::ZERO) - r#yield.unwrap_or(Kilowatts::ZERO)
        })
        .collect_array()
        .unwrap();

    let solution = Solver::builder()
        .grid_rates(&grid_rates)
        .residual_energy(residual_energy)
        .capacity(total_capacity)
        .battery_args(hunt_args.battery)
        .purchase_fee(hunt_args.purchase_fee)
        .stand_by_power(stand_by_power)
        .now(now)
        .working_modes(working_modes)
        .solve();

    let profit = solution.summary.profit();
    #[allow(clippy::cast_precision_loss)]
    let daily_profit = profit / (grid_rates.len() as f64 / 24.0);
    info!(
        "Optimized",
        net_loss = solution.summary.net_loss,
        without_battery = solution.summary.net_loss_without_battery,
        profit = profit,
        daily_profit = daily_profit,
    );
    println!("{}", render_steps(&grid_rates, &solution.steps, hunt_args.battery, total_capacity));

    let schedule: Series<_, _> =
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

async fn burrow_fox_ess(
    fox_ess: &foxess::Api,
    serial_number: &str,
    args: BurrowFoxEssArgs,
) -> Result {
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
