#![allow(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]

mod api;
mod cli;
mod core;
mod prelude;
mod quantity;
mod statistics;
mod tables;

use chrono::Timelike;
use clap::Parser;
use itertools::Itertools;

use crate::{
    api::{foxess, heartbeat, nextenergy},
    cli::{Args, BurrowCommand, BurrowFoxEssArgs, BurrowFoxEssCommand, Command, HuntArgs},
    core::{
        series::Series,
        solver::{Solver, conditions::Conditions},
    },
    prelude::*,
    quantity::power::Kilowatts,
    tables::{build_steps_table, build_time_slot_sequence_table},
};

#[tokio::main]
async fn main() -> Result {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt().without_time().with_target(false).compact().init();

    let args = Args::parse();

    match args.command {
        Command::Hunt(hunt_args) => {
            hunt(*hunt_args).await?;
        }
        Command::Burrow(burrow_args) => match burrow_args.command {
            BurrowCommand::Statistics(statistics_args) => {
                let home_assistant = statistics_args.home_assistant.connection.try_new_client()?;
                let statistics = home_assistant
                    .get_statistics(
                        &statistics_args.home_assistant.entity_id,
                        &statistics_args.home_assistant.history_period(),
                    )
                    .await?;
                let contents = toml::to_string_pretty(&statistics)?;
                if let Some(path) = statistics_args.output_file {
                    std::fs::write(path, contents)?;
                } else {
                    println!("{contents}");
                }
            }

            BurrowCommand::FoxEss(burrow_args) => {
                burrow_fox_ess(burrow_args).await?;
            }
        },
    }

    info!("Done!");
    Ok(())
}

#[instrument(skip_all)]
async fn hunt(args: HuntArgs) -> Result {
    let fox_ess = foxess::Api::try_new(args.fox_ess_api.api_key.clone())?;
    let working_modes = args.working_modes();
    let home_assistant = args.home_assistant.connection.try_new_client()?;
    let history_period = args.home_assistant.history_period();

    let grid_rates: Series<_, _> =
        nextenergy::Api::try_new()?.get_hourly_rates_48h(*history_period.end()).await?.collect();
    ensure!(!grid_rates.is_empty());
    info!(len = grid_rates.len(), "Fetched energy rates");

    let residual_energy =
        fox_ess.get_device_variables(&args.fox_ess_api.serial_number).await?.residual_energy;
    let total_capacity =
        fox_ess.get_device_details(&args.fox_ess_api.serial_number).await?.total_capacity();
    info!(?residual_energy, ?total_capacity, "Fetched battery details");

    let conditions = {
        let statistics =
            home_assistant.get_statistics(&args.home_assistant.entity_id, &history_period).await?;
        grid_rates
            .into_iter()
            .map(|(time_range, grid_rate)| {
                let hour = time_range.start.hour() as usize;
                (
                    time_range,
                    Conditions {
                        grid_rate,
                        stand_by_power: statistics.household.hourly_stand_by_power[hour]
                            .unwrap_or(Kilowatts::ZERO),
                    },
                )
            })
            .collect_vec()
    };

    let solution = Solver::builder()
        .conditions(&conditions)
        .working_modes(working_modes)
        .residual_energy(residual_energy)
        .capacity(total_capacity)
        .battery_args(args.battery)
        .purchase_fee(args.purchase_fee)
        .now(*history_period.end())
        .solve()
        .context("no solution found, try allowing additional working modes")?;
    println!("{}", build_steps_table(&conditions, &solution.steps, args.battery, total_capacity));

    let schedule: Series<_, _> =
        solution.steps.into_iter().map(|(time, step)| (time, step.working_mode)).collect();
    let time_slot_sequence = foxess::TimeSlotSequence::from_schedule(&schedule, &args.battery)?;
    println!("{}", build_time_slot_sequence_table(&time_slot_sequence));

    if !args.scout {
        fox_ess.set_schedule(&args.fox_ess_api.serial_number, time_slot_sequence.as_ref()).await?;
    }

    if let Some(heartbeat_url) = args.heartbeat_url {
        heartbeat::send(heartbeat_url).await;
    }

    Ok(())
}

#[instrument(skip_all)]
async fn burrow_fox_ess(args: BurrowFoxEssArgs) -> Result {
    let fox_ess = foxess::Api::try_new(args.fox_ess_api.api_key)?;

    match args.command {
        BurrowFoxEssCommand::DeviceDetails => {
            let details = fox_ess.get_device_details(&args.fox_ess_api.serial_number).await?;
            info!(total_capacity = ?details.total_capacity(), "Gotcha");
        }

        BurrowFoxEssCommand::DeviceVariables => {
            let variables = fox_ess.get_device_variables(&args.fox_ess_api.serial_number).await?;
            info!(?variables.residual_energy, "Gotcha");
        }

        BurrowFoxEssCommand::RawDeviceVariables => {
            let response =
                fox_ess.get_devices_variables_raw(&[&args.fox_ess_api.serial_number]).await?;
            info!("Gotcha!");
            for device in response {
                for variable in device.variables {
                    info!(
                        serial_number = &device.serial_number,
                        name = variable.name,
                        description = variable.description,
                        unit = variable.unit,
                        value = variable.value.to_string(),
                        "Variable",
                    );
                }
            }
        }

        BurrowFoxEssCommand::Schedule => {
            let schedule = fox_ess.get_schedule(&args.fox_ess_api.serial_number).await?;
            info!(schedule.is_enabled, "Gotcha");
            println!("{}", build_time_slot_sequence_table(&schedule.groups));
        }
    }
    Ok(())
}
