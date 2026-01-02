#![allow(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]

mod api;
mod cli;
mod core;
mod prelude;
mod quantity;
mod statistics;
mod tables;

use chrono::{Local, TimeDelta, Timelike};
use clap::{Parser, crate_version};
use itertools::Itertools;

use crate::{
    api::{foxess, heartbeat},
    cli::{
        Args,
        BurrowCommand,
        BurrowFoxEssArgs,
        BurrowFoxEssCommand,
        BurrowStatisticsArgs,
        Command,
        HuntArgs,
    },
    core::{
        series::{Aggregate, Extend},
        solver::Solver,
    },
    prelude::*,
    quantity::interval::Interval,
    statistics::{Statistics, energy::EnergyStatistics, rates::ProviderStatistics},
    tables::{build_steps_table, build_time_slot_sequence_table},
};

fn main() -> Result {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt().without_time().compact().init();
    info!(version = crate_version!(), "Starting…");

    let args = Args::parse();

    match args.command {
        Command::Hunt(args) => {
            hunt(&args)?;
        }
        Command::Burrow(args) => match args.command {
            BurrowCommand::Statistics(statistics_args) => {
                burrow_statistics(&statistics_args)?;
            }
            BurrowCommand::FoxEss(args) => {
                burrow_fox_ess(args)?;
            }
        },
    }

    if let Some(heartbeat_url) = args.heartbeat_url
        && let Err(error) = heartbeat::send(heartbeat_url)
    {
        warn!("Failed to send the heartbeat: {error:#}");
    }
    info!("Done!");
    Ok(())
}

#[instrument(skip_all)]
fn hunt(args: &HuntArgs) -> Result {
    let statistics = Statistics::read_from(&args.statistics_path)?;
    info!(?statistics.generated_at);
    info!(parasitic_load = ?statistics.energy.battery.parasitic_load);
    info!(charging_efficiency = format!("{:.3}", statistics.energy.battery.charging_efficiency));
    info!(
        discharging_efficiency = format!("{:.3}", statistics.energy.battery.discharging_efficiency)
    );
    info!(
        round_trip_efficiency = format!("{:.3}", statistics.energy.battery.round_trip_efficiency())
    );

    let fox_ess = foxess::Api::new(args.fox_ess_api.api_key.clone());
    let working_modes = args.working_modes();

    let now = Local::now().with_nanosecond(0).unwrap();
    let mut grid_rates = args.provider.get_upcoming_rates(now)?;

    ensure!(!grid_rates.is_empty());
    info!(len = grid_rates.len(), "Fetched energy rates");

    if let Some(provider_statistics) = statistics.providers.get(&args.provider) {
        let look_ahead_timestamp = now + TimeDelta::from_std(*args.look_ahead_duration)?;
        info!(?look_ahead_timestamp, "Using median rates until the forecast horizon");
        grid_rates.extend_grid_rates(
            args.provider,
            provider_statistics,
            Interval::new(now, look_ahead_timestamp),
        );
    }

    let residual_energy =
        fox_ess.get_device_variables(&args.fox_ess_api.serial_number)?.residual_energy;
    let total_capacity =
        fox_ess.get_device_details(&args.fox_ess_api.serial_number)?.total_capacity();
    info!(?residual_energy, ?total_capacity, "Fetched battery details");

    let solution = Solver::builder()
        .grid_rates(&grid_rates)
        .hourly_stand_by_power(&statistics.energy.household.hourly_stand_by_power)
        .working_modes(working_modes)
        .initial_residual_energy(residual_energy)
        .capacity(total_capacity)
        .battery_args(args.battery_args)
        .battery_parameters(statistics.energy.battery)
        .purchase_fee(args.provider.purchase_fee())
        .now(now)
        .solve()
        .context("no solution found, try allowing additional working modes")?;
    let steps = solution.backtrack().collect_vec();
    println!("{}", build_steps_table(&steps, args.battery_args));

    let schedule = steps.into_iter().map(|step| (step.interval, step.working_mode)).collect_vec();
    let time_slot_sequence =
        foxess::TimeSlotSequence::from_schedule(schedule, now, &args.battery_args)?;
    println!("{}", build_time_slot_sequence_table(&time_slot_sequence));

    if !args.scout {
        fox_ess.set_schedule(&args.fox_ess_api.serial_number, time_slot_sequence.as_ref())?;
    }

    Ok(())
}

#[instrument(skip_all)]
fn burrow_statistics(args: &BurrowStatisticsArgs) -> Result {
    let history_period = args.home_assistant.history_period();
    let mut statistics = Statistics::read_from(&args.statistics_path)?;

    statistics.generated_at = *history_period.end();
    statistics.energy = args
        .home_assistant
        .connection
        .new_client()
        .get_energy_history(&args.home_assistant.entity_id, &history_period)?
        .into_iter()
        .collect::<EnergyStatistics>();
    let rates =
        statistics.providers.entry(args.provider).or_insert_with(ProviderStatistics::default);
    for (interval, rate) in args.provider.get_rates(Local::now().date_naive())? {
        rates.history.insert(interval.start, rate);
    }
    rates.history.retain(|start_time, _| start_time >= history_period.start());
    rates.medians = rates
        .history
        .iter()
        .map(|(start_time, rate)| (start_time.time(), *rate))
        .into_group_map()
        .into_iter()
        .filter_map(|(time, rates)| Some((time, rates.median()?)))
        .collect();

    statistics.write_to(&args.statistics_path)?;
    Ok(())
}

#[instrument(skip_all)]
fn burrow_fox_ess(args: BurrowFoxEssArgs) -> Result {
    let fox_ess = foxess::Api::new(args.fox_ess_api.api_key);

    match args.command {
        BurrowFoxEssCommand::DeviceDetails => {
            let details = fox_ess.get_device_details(&args.fox_ess_api.serial_number)?;
            info!(total_capacity = ?details.total_capacity(), "Gotcha");
        }

        BurrowFoxEssCommand::DeviceVariables => {
            let variables = fox_ess.get_device_variables(&args.fox_ess_api.serial_number)?;
            info!(?variables.residual_energy, "Gotcha");
        }

        BurrowFoxEssCommand::RawDeviceVariables => {
            let response = fox_ess.get_devices_variables_raw(&[&args.fox_ess_api.serial_number])?;
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
            let schedule = fox_ess.get_schedule(&args.fox_ess_api.serial_number)?;
            info!(schedule.is_enabled, "Gotcha");
            println!("{}", build_time_slot_sequence_table(&schedule.groups));
        }
    }
    Ok(())
}
