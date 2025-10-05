#![allow(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]

mod api;
mod cli;
mod core;
mod prelude;
mod quantity;
mod render;

use std::ops::RangeInclusive;

use chrono::{DateTime, Local, TimeDelta};
use clap::Parser;
use itertools::Itertools;
use logfire::config::{ConsoleOptions, SendToLogfire};
use serde::de::IgnoredAny;
use tracing::level_filters::LevelFilter;

use crate::{
    api::{
        foxess,
        heartbeat,
        home_assistant,
        home_assistant::battery::{BatteryState, BatteryStateAttributes},
        nextenergy,
    },
    cli::{Args, BurrowCommand, BurrowFoxEssArgs, BurrowFoxEssCommand, Command, HuntArgs},
    core::{
        series::{
            AverageHourly,
            Differentiate,
            Point,
            ResampleHourly,
            Series,
            TryEstimateBatteryParameters,
        },
        solver::Solver,
    },
    prelude::*,
    quantity::{energy::KilowattHours, power::Kilowatts},
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
                burrow_fox_ess(&fox_ess, &args.fox_ess_api.serial_number, burrow_args).await?;
            }
            BurrowCommand::BatteryDifferentials(history_args) => {
                let now = Local::now();
                let home_assistant_period =
                    (now - TimeDelta::days(history_args.home_assistant.n_history_days))..=now;
                let battery_differentials = history_args
                    .home_assistant
                    .connection
                    .try_new_client()?
                    .get_battery_differentials(
                        &history_args.home_assistant.battery_state_entity_id,
                        &home_assistant_period,
                    )
                    .await?
                    .collect_vec();
                println!("{}", serde_json::to_string_pretty(&battery_differentials)?);
            }
        },
    }

    info!("Done!");
    Ok(())
}

async fn hunt(fox_ess: &foxess::Api, serial_number: &str, hunt_args: HuntArgs) -> Result {
    let home_assistant = hunt_args.home_assistant.connection.try_new_client()?;

    let now = Local::now();
    let home_assistant_period =
        (now - TimeDelta::days(hunt_args.home_assistant.n_history_days))..=now;

    let grid_rates: Series<_, _> =
        nextenergy::Api::try_new()?.get_hourly_rates_48h(now).await?.collect();
    ensure!(!grid_rates.is_empty());
    info!("Fetched energy rates", len = grid_rates.len());

    let residual_energy = fox_ess.get_device_variables(serial_number).await?.residual_energy;
    let total_capacity = fox_ess.get_device_details(serial_number).await?.total_capacity();
    info!("Fetched battery details", residual_energy, total_capacity);

    // Fetch the battery state history and estimate the parameters:
    let battery_parameters = home_assistant
        .get_battery_differentials(
            &hunt_args.home_assistant.battery_state_entity_id,
            &home_assistant_period,
        )
        .await?
        .try_estimate_battery_parameters()
        .unwrap_or_default();

    // Calculate the stand-by power:
    let stand_by_usage = home_assistant
        .get_history::<KilowattHours, IgnoredAny>(
            &hunt_args.home_assistant.total_usage_entity_id,
            &home_assistant_period,
        )
        .await?
        .into_iter()
        .map(|state| (state.last_changed_at, state.value))
        .resample_hourly()
        .differentiate()
        .average_hourly();
    let solar_yield = home_assistant
        .get_history::<KilowattHours, IgnoredAny>(
            &hunt_args.home_assistant.solar_yield_entity_id,
            &home_assistant_period,
        )
        .await?
        .into_iter()
        .map(|state| (state.last_changed_at, state.value))
        .resample_hourly()
        .differentiate()
        .average_hourly();
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
        .battery_parameters(battery_parameters)
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
    println!(
        "{}",
        try_render_steps(&grid_rates, &solution.steps, hunt_args.battery, total_capacity)?
    );

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

impl home_assistant::Api {
    async fn get_battery_differentials(
        &self,
        entity_id: &str,
        period: &RangeInclusive<DateTime<Local>>,
    ) -> Result<impl Iterator<Item = Point<DateTime<Local>, BatteryState<Kilowatts>>>> {
        Ok(self
            .get_history::<KilowattHours, BatteryStateAttributes<KilowattHours>>(entity_id, period)
            .await?
            .into_iter()
            .map(|state| {
                (
                    state.last_changed_at,
                    BatteryState { residual_energy: state.value, attributes: state.attributes },
                )
            })
            .differentiate())
    }
}
