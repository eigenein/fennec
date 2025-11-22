#![allow(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]

mod api;
mod cli;
mod core;
mod prelude;
mod quantity;
mod statistics;
mod tables;

use std::iter::once;

use chrono::{Local, Timelike};
use clap::{Parser, crate_version};
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
    statistics::Statistics,
    tables::{build_steps_table, build_time_slot_sequence_table},
};

#[tokio::main]
async fn main() -> Result {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt().without_time().compact().init();
    info!(version = crate_version!(), "Starting…");

    let args = Args::parse();

    match args.command {
        Command::Hunt(hunt_args) => {
            hunt(*hunt_args).await?;
        }
        Command::Burrow(burrow_args) => match burrow_args.command {
            BurrowCommand::Statistics(statistics_args) => {
                statistics_args
                    .home_assistant
                    .connection
                    .try_new_client()?
                    .get_energy_history(
                        &statistics_args.home_assistant.entity_id,
                        &statistics_args.home_assistant.history_period(),
                    )
                    .await?
                    .into_iter()
                    .collect::<Statistics>()
                    .write_to(&statistics_args.output_path)?;
            }

            BurrowCommand::FoxEss(burrow_args) => {
                burrow_fox_ess(burrow_args).await?;
            }
        },
    }

    if let Some(heartbeat_url) = args.heartbeat_url {
        heartbeat::send(heartbeat_url).await;
    }
    info!("Done!");
    Ok(())
}

#[instrument(skip_all)]
async fn hunt(args: HuntArgs) -> Result {
    let statistics = Statistics::read_from(&args.statistics_path)?;
    info!(?statistics.generated_at);
    info!(parasitic_load = ?statistics.battery.parasitic_load);
    info!(charging_efficiency = format!("{:.3}", statistics.battery.charging_efficiency));
    info!(discharging_efficiency = format!("{:.3}", statistics.battery.discharging_efficiency));
    info!(round_trip_efficiency = format!("{:.3}", statistics.battery.round_trip_efficiency()));

    let fox_ess = foxess::Api::try_new(args.fox_ess_api.api_key.clone())?;
    let working_modes = args.working_modes();

    let now = Local::now().with_nanosecond(0).unwrap();
    let grid_rates: Series<_, _> =
        nextenergy::Api::try_new()?.get_hourly_rates_48h(now.date_naive()).await?.collect();
    ensure!(!grid_rates.is_empty());
    info!(len = grid_rates.len(), "Fetched energy rates");

    let residual_energy =
        fox_ess.get_device_variables(&args.fox_ess_api.serial_number).await?.residual_energy;
    let total_capacity =
        fox_ess.get_device_details(&args.fox_ess_api.serial_number).await?.total_capacity();
    info!(?residual_energy, ?total_capacity, "Fetched battery details");

    let conditions = grid_rates
        .into_iter()
        .map(|(time_range, grid_rate)| {
            let hour = time_range.start.hour() as usize;
            let stand_by_power =
                statistics.household.hourly_stand_by_power[hour].unwrap_or(Kilowatts::ZERO);
            (time_range, Conditions { grid_rate, stand_by_power })
        })
        .flat_map(|(time_range, conditions)| {
            // TODO: extract and test:
            let step = (time_range.end - time_range.start) / (i32::from(args.n_hour_splits) + 1);
            (0..args.n_hour_splits)
                .map(move |i| {
                    // First N time spans:
                    let i = i32::from(i);
                    ((time_range.start + step * i)..(time_range.start + step * (i + 1)), conditions)
                })
                .chain(once(
                    // Last time span:
                    (
                        (time_range.start + step * i32::from(args.n_hour_splits))..time_range.end,
                        conditions,
                    ),
                ))
        })
        .filter(move |(time_range, _)| time_range.end > now)
        .collect_vec();
    let solution = Solver::builder()
        .conditions(&conditions)
        .working_modes(working_modes)
        .residual_energy(residual_energy)
        .capacity(total_capacity)
        .battery_args(args.battery_args)
        .battery_parameters(statistics.battery)
        .purchase_fee(args.purchase_fee)
        .now(now)
        .solve()
        .context("no solution found, try allowing additional working modes")?;
    println!(
        "{}",
        build_steps_table(&conditions, &solution.steps, args.battery_args, total_capacity),
    );

    let schedule: Series<_, _> =
        solution.steps.into_iter().map(|(time, step)| (time, step.working_mode)).collect();
    let time_slot_sequence =
        foxess::TimeSlotSequence::from_schedule(schedule, now, &args.battery_args)?;
    println!("{}", build_time_slot_sequence_table(&time_slot_sequence));

    if !args.scout {
        fox_ess.set_schedule(&args.fox_ess_api.serial_number, time_slot_sequence.as_ref()).await?;
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
