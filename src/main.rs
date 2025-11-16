#![allow(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]

mod api;
mod cli;
mod core;
mod prelude;
mod quantity;
mod tables;

use chrono::{Local, TimeDelta, Timelike};
use clap::Parser;
use itertools::Itertools;

use crate::{
    api::{foxess, heartbeat, nextenergy},
    cli::{Args, BurrowCommand, BurrowFoxEssArgs, BurrowFoxEssCommand, Command, HuntArgs},
    core::{
        series::{Aggregate, Differentiate, Series},
        solver::{Solver, conditions::Conditions},
    },
    prelude::*,
    quantity::{energy::KilowattHours, power::Kilowatts},
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
    let now = Local::now();
    let history_period = (now - TimeDelta::days(args.home_assistant.n_history_days))..=now;

    let grid_rates: Series<_, _> =
        nextenergy::Api::try_new()?.get_hourly_rates_48h(now).await?.collect();
    ensure!(!grid_rates.is_empty());
    info!(len = grid_rates.len(), "Fetched energy rates");

    let residual_energy =
        fox_ess.get_device_variables(&args.fox_ess_api.serial_number).await?.residual_energy;
    let total_capacity =
        fox_ess.get_device_details(&args.fox_ess_api.serial_number).await?.total_capacity();
    info!(?residual_energy, ?total_capacity, "Fetched battery details");

    let conditions = {
        let median_stand_by_power = home_assistant
            .get_energy_history(&args.home_assistant.entity_id, &history_period)
            .await?
            .into_iter()
            .map(|state| {
                (
                    state.last_changed_at,
                    state.total_net_usage
                        - state.attributes.total_solar_yield.unwrap_or(KilowattHours::ZERO),
                )
            })
            .differentiate()
            .median_hourly();
        grid_rates
            .into_iter()
            .map(|(time_range, grid_rate)| {
                let hour = time_range.start.hour() as usize;
                (
                    time_range,
                    Conditions {
                        grid_rate,
                        stand_by_power: median_stand_by_power[hour].unwrap_or(Kilowatts::ZERO),
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
        .now(now)
        .solve()
        .context("no solution found, try allowing additional working modes")?;

    let profit = solution.profit();

    #[allow(clippy::cast_precision_loss)]
    let daily_profit = profit / (conditions.len() as f64 / 24.0);

    info!(
        net_loss = ?solution.net_loss,
        without_battery = ?solution.net_loss_without_battery,
        ?profit,
        ?daily_profit,
        "Optimized",
    );
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
