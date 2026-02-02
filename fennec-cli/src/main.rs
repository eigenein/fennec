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

use clap::{Parser, crate_version};

use crate::{
    api::foxess,
    cli::{
        Args,
        BurrowCommand,
        BurrowFoxEssArgs,
        BurrowFoxEssCommand,
        BurrowStatisticsArgs,
        Command,
        hunt,
        log,
    },
    core::interval::Interval,
    db::{
        Db,
        battery_log::BatteryLogs,
        state::{HourlyStandByPower, States},
    },
    prelude::*,
    statistics::battery::BatteryEfficiency,
    tables::build_time_slot_sequence_table,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt().without_time().compact().init();
    info!(version = crate_version!(), "starting…");

    let args = Args::parse();

    match args.command {
        Command::Hunt(args) => hunt(&args).await,
        Command::Log(args) => log(*args).await,
        Command::Burrow(args) => match args.command {
            BurrowCommand::Statistics(args) => {
                burrow_statistics(&args).await?;
                args.heartbeat.send().await;
                Ok(())
            }
            BurrowCommand::Battery(args) => {
                let battery_logs = BatteryLogs::from(&Db::with_uri(&args.db.uri).await?);
                let _ = BatteryEfficiency::try_estimate(
                    battery_logs.find(Interval::try_since(args.estimation.duration())?).await?,
                )
                .await?;
                Ok(())
            }
            BurrowCommand::FoxEss(args) => burrow_fox_ess(args).await,
        },
    }
}

/// TODO: move to a separate module.
#[instrument(skip_all)]
async fn burrow_statistics(args: &BurrowStatisticsArgs) -> Result {
    let history_period = args.home_assistant.history_period();
    let hourly_stand_by_power = args
        .home_assistant
        .connection
        .new_client()
        .get_energy_history(&args.home_assistant.entity_id, &history_period)?
        .into_iter()
        .map(|state| (state.last_changed_at, state))
        .collect::<HourlyStandByPower>();
    States::from(&Db::with_uri(&args.db.uri).await?).set(&hourly_stand_by_power).await?;
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
