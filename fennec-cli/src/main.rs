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

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use chrono::{Local, Timelike};
use clap::{Parser, crate_version};
use itertools::Itertools;
use tokio::time::sleep;
use turso::transaction::{Transaction, TransactionBehavior};

use crate::{
    api::{foxess, homewizard, modbus},
    cli::{
        Args,
        BurrowCommand,
        BurrowFoxEssArgs,
        BurrowFoxEssCommand,
        BurrowStatisticsArgs,
        Command,
        HuntArgs,
        LogArgs,
    },
    core::solver::Solver,
    db::{
        Db,
        battery_log::BatteryLog,
        legacy_db::LegacyDb,
        legacy_key::LegacyKey,
        scalars::LegacyScalars,
        state::{BatteryResidualEnergy, HourlyStandByPower},
    },
    prelude::*,
    quantity::energy::MilliwattHours,
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
            args.heartbeat.send().await;
        }
        Command::Log(args) => {
            log(*args).await?;
        }
        Command::Burrow(args) => match args.command {
            BurrowCommand::Statistics(args) => {
                burrow_statistics(&args).await?;
                args.heartbeat.send().await;
            }
            BurrowCommand::Battery(args) => {
                let _ = BatteryEfficiency::try_estimate_from(
                    &LegacyDb::connect(&args.db.path, false).await?,
                    args.estimation.duration(),
                )
                .await?;
            }
            BurrowCommand::FoxEss(args) => {
                burrow_fox_ess(args).await?;
            }
        },
    }

    info!("done!");
    Ok(())
}

/// TODO: move to a separate module.
#[instrument(skip_all)]
async fn hunt(args: &HuntArgs) -> Result {
    let statistics = Statistics::read_from(&args.statistics_path)?;
    info!(?statistics.generated_at);

    let fox_ess = foxess::Api::new(args.fox_ess_api.api_key.clone())?;
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

    let battery_efficiency = BatteryEfficiency::try_estimate_from(
        &LegacyDb::connect(&args.db.path, false).await?,
        args.estimation.duration(),
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
        .degradation_rate(args.degradation_rate)
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
        fox_ess.set_schedule(&args.fox_ess_api.serial_number, time_slot_sequence.as_ref()).await?;
    }

    Ok(())
}

/// TODO: move to a separate module and split the battery and household loggers.
/// TODO: separate loops and intervals for battery and P1 loggers.
async fn log(args: LogArgs) -> Result {
    // TODO: this one should be independently fallible:
    // let total_energy_meter = homewizard::Client::new(args.total_energy_meter_url)?;

    let polling_interval: Duration = args.polling_interval();
    let battery_energy_meter = homewizard::Client::new(args.battery_energy_meter_url)?;
    let mut battery = modbus::Client::connect(&args.battery_connection).await?;
    drop(LegacyDb::connect(&args.db.path, true).await?);
    let db = Db::with_uri(args.db.uri).await?;

    // TODO: implement proper signal handling with cancelling the `sleep` call.
    let should_terminate = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&should_terminate))?;

    while !should_terminate.load(Ordering::Relaxed) {
        let now = Local::now();

        let (battery_measurement, battery_state) = {
            tokio::try_join!(
                battery_energy_meter.get_measurement(),
                battery.read_energy_state(args.battery_registers),
            )?
        };

        let mut legacy_db = LegacyDb::connect(&args.db.path, false).await?;
        let tx = Transaction::new(&mut legacy_db, TransactionBehavior::Deferred).await?;

        let last_known_residual =
            LegacyScalars(&tx).select::<MilliwattHours>(LegacyKey::BatteryResidualEnergy).await?;
        if let Some(last_known_residual) = last_known_residual
            && (last_known_residual != battery_state.residual_millis())
        {
            BatteryLog::builder()
                .timestamp(now)
                .residual_energy(battery_state.residual_millis().into())
                .meter(battery_measurement)
                .build()
                .insert_into(&tx)
                .await?;
        }
        LegacyScalars(&tx)
            .upsert(LegacyKey::BatteryResidualEnergy, battery_state.residual_millis())
            .await?;
        db.states().upsert(&BatteryResidualEnergy::from(battery_state.residual_millis())).await?;

        tx.commit().await?;
        drop(legacy_db);

        args.heartbeat.send().await;
        sleep(polling_interval).await;
    }

    Ok(())
}

/// TODO: move to a separate module.
#[instrument(skip_all)]
async fn burrow_statistics(args: &BurrowStatisticsArgs) -> Result {
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
    Db::with_uri(&args.db.uri)
        .await?
        .states()
        .upsert(&HourlyStandByPower::from(statistics.energy.household.hourly_stand_by_power))
        .await?;
    statistics.write_to(&args.statistics_path).context("failed to write the statistics file")?;
    Ok(())
}

/// TODO: move to a separate module.
#[instrument(skip_all)]
async fn burrow_fox_ess(args: BurrowFoxEssArgs) -> Result {
    let fox_ess = foxess::Api::new(args.fox_ess_api.api_key)?;

    match args.command {
        BurrowFoxEssCommand::Schedule => {
            let schedule = fox_ess.get_schedule(&args.fox_ess_api.serial_number).await?;
            info!(schedule.is_enabled, "gotcha");
            println!("{}", build_time_slot_sequence_table(&schedule.groups));
        }
    }

    Ok(())
}
