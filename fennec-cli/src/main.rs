#![allow(clippy::doc_markdown)]
#![doc = include_str!("../../README.md")]

mod api;
mod cli;
mod core;
mod db;
mod fmt;
mod prelude;
mod quantity;
mod statistics;
mod tables;

use chrono::{Local, Timelike};
use clap::{Parser, crate_version};
use itertools::Itertools;

use crate::{
    api::{foxess, heartbeat, homewizard, modbus},
    cli::{
        Args,
        BurrowCommand,
        BurrowFoxEssArgs,
        BurrowFoxEssCommand,
        BurrowStatisticsArgs,
        Command,
        HuntArgs,
    },
    core::solver::Solver,
    db::{measurement::Measurement, measurements::Measurements},
    prelude::*,
    statistics::{Statistics, battery::BatteryEfficiency, household::EnergyStatistics},
    tables::{build_steps_table, build_time_slot_sequence_table},
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt().without_time().compact().init();
    info!(version = crate_version!(), "starting…");

    let args = Args::parse();

    match args.command {
        Command::Hunt(args) => {
            hunt(&args).await?;
        }
        Command::Log(args) => {
            let measurement = {
                let total_energy_meter = homewizard::Client::new(args.total_energy_meter_url)?;
                let battery_energy_meter = homewizard::Client::new(args.battery_energy_meter_url)?;
                let mut battery = modbus::Client::connect(&args.battery_connection).await?;
                let (total_measurement, battery_measurement, battery_state) = tokio::try_join!(
                    total_energy_meter.get_measurement(),
                    battery_energy_meter.get_measurement(),
                    battery.read_energy_state(args.battery_registers),
                )?;
                Measurement::builder()
                    .timestamp(Local::now())
                    .total(total_measurement)
                    .battery(battery_measurement)
                    .residual_energy(battery_state.residual())
                    .build()
            };
            Measurements(&*args.database.connect().await?).upsert(&measurement).await?;
        }
        Command::Burrow(args) => match args.command {
            BurrowCommand::Statistics(args) => {
                burrow_statistics(&args)?;
            }
            BurrowCommand::Battery(args) => {
                let _ = BatteryEfficiency::try_estimate_from(
                    &args.database.connect().await?,
                    args.estimation.duration.into(),
                )
                .await?;
            }
            BurrowCommand::FoxEss(args) => {
                burrow_fox_ess(args)?;
            }
        },
    }

    if let Some(heartbeat_url) = args.heartbeat_url
        && let Err(error) = heartbeat::send(heartbeat_url).await
    {
        warn!("failed to send the heartbeat: {error:#}");
    }
    info!("done!");
    Ok(())
}

#[instrument(skip_all)]
async fn hunt(args: &HuntArgs) -> Result {
    let statistics = Statistics::read_from(&args.statistics_path)?;
    info!(?statistics.generated_at);

    let fox_ess = foxess::Api::new(args.fox_ess_api.api_key.clone());
    let working_modes = args.working_modes();

    let now = Local::now().with_nanosecond(0).unwrap();
    let grid_rates = args.provider.get_upcoming_rates(now).await?;

    ensure!(!grid_rates.is_empty());
    info!(len = grid_rates.len(), "fetched energy rates");

    let battery_state = modbus::Client::connect(&args.battery.connection)
        .await?
        .read_battery_state(args.battery.registers)
        .await?;
    let min_state_of_charge = battery_state.settings.min_state_of_charge;
    let max_state_of_charge = battery_state.settings.max_state_of_charge;
    info!(
        residual_energy = ?battery_state.energy.residual(),
        actual_capacity = ?battery_state.energy.actual_capacity(),
        min_soc = ?battery_state.settings.min_state_of_charge,
        max_soc = ?battery_state.settings.max_state_of_charge,
        "fetched battery state",
    );

    let battery_efficiency = BatteryEfficiency::try_estimate_from(
        &args.database.connect().await?,
        args.estimation.duration.into(),
    )
    .await?;

    let solution = Solver::builder()
        .grid_rates(&grid_rates)
        .hourly_stand_by_power(&statistics.energy.household.hourly_stand_by_power)
        .working_modes(working_modes)
        .battery_state(battery_state)
        .battery_power_limits(args.battery.power_limits)
        .battery_efficiency(battery_efficiency)
        .purchase_fee(args.provider.purchase_fee())
        .now(now)
        .solve()
        .context("no solution found, try allowing additional working modes")?;
    let steps = solution.backtrack().collect_vec();
    println!("{}", build_steps_table(&steps, args.battery.power_limits.discharging_power));

    let schedule = steps.into_iter().map(|step| (step.interval, step.working_mode)).collect_vec();
    let time_slot_sequence = foxess::TimeSlotSequence::from_schedule(
        schedule,
        now,
        args.battery.power_limits,
        min_state_of_charge,
        max_state_of_charge,
    )?;
    println!("{}", build_time_slot_sequence_table(&time_slot_sequence));

    if !args.scout {
        fox_ess.set_schedule(&args.fox_ess_api.serial_number, time_slot_sequence.as_ref())?;
    }

    Ok(())
}

#[instrument(skip_all)]
fn burrow_statistics(args: &BurrowStatisticsArgs) -> Result {
    let history_period = args.home_assistant.history_period();
    let statistics = Statistics {
        generated_at: *history_period.end(),
        energy: args
            .home_assistant
            .connection
            .new_client()
            .get_energy_history(&args.home_assistant.entity_id, &history_period)?
            .into_iter()
            .collect::<EnergyStatistics>(),
    };
    statistics.write_to(&args.statistics_path).context("failed to write the statistics file")?;
    Ok(())
}

#[instrument(skip_all)]
fn burrow_fox_ess(args: BurrowFoxEssArgs) -> Result {
    let fox_ess = foxess::Api::new(args.fox_ess_api.api_key);

    match args.command {
        BurrowFoxEssCommand::Schedule => {
            let schedule = fox_ess.get_schedule(&args.fox_ess_api.serial_number)?;
            info!(schedule.is_enabled, "gotcha");
            println!("{}", build_time_slot_sequence_table(&schedule.groups));
        }
    }

    Ok(())
}
