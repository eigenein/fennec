use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{
    api::foxess,
    cli::{
        HomeAssistantArgs,
        db::DbArgs,
        estimation::EstimationArgs,
        foxess::FoxEssApiArgs,
        heartbeat::HeartbeatArgs,
    },
    core::interval::Interval,
    db::{
        battery_log::BatteryLogs,
        state::{HourlyStandByPower, States},
    },
    prelude::*,
    statistics::battery::BatteryEfficiency,
    tables::build_time_slot_sequence_table,
};

#[derive(Parser)]
pub struct BurrowArgs {
    #[command(subcommand)]
    command: BurrowCommand,
}

impl BurrowArgs {
    pub async fn burrow(self) -> Result {
        match self.command {
            BurrowCommand::Statistics(args) => args.burrow().await,
            BurrowCommand::Battery(args) => args.burrow().await,
            BurrowCommand::FoxEss(args) => args.burrow().await,
        }
    }
}

#[derive(Subcommand)]
pub enum BurrowCommand {
    /// Gather consumption and battery statistics.
    Statistics(Box<BurrowStatisticsArgs>),

    /// Estimate battery efficiency parameters.
    Battery(BurrowBatteryArgs),

    /// Test FoxESS Cloud API connectivity.
    FoxEss(BurrowFoxEssArgs),
}

#[derive(Parser)]
pub struct BurrowStatisticsArgs {
    #[clap(flatten)]
    home_assistant: HomeAssistantArgs,

    #[clap(long, env = "STATISTICS_PATH", default_value = "statistics.toml")]
    statistics_path: PathBuf,

    #[clap(flatten)]
    heartbeat: HeartbeatArgs,

    #[clap(flatten)]
    db: DbArgs,
}

impl BurrowStatisticsArgs {
    #[instrument(skip_all)]
    async fn burrow(self) -> Result {
        let history_period = self.home_assistant.history_period();
        let hourly_stand_by_power = self
            .home_assistant
            .connection
            .new_client()
            .get_energy_history(&self.home_assistant.entity_id, &history_period)?
            .into_iter()
            .map(|state| (state.last_changed_at, state))
            .collect::<HourlyStandByPower>();
        States::from(&self.db.connect().await?).set(&hourly_stand_by_power).await?;
        self.heartbeat.send().await;
        Ok(())
    }
}

#[derive(Parser)]
pub struct BurrowBatteryArgs {
    #[clap(flatten)]
    db: DbArgs,

    #[clap(flatten)]
    estimation: EstimationArgs,
}

impl BurrowBatteryArgs {
    async fn burrow(self) -> Result {
        let battery_logs = BatteryLogs::from(&self.db.connect().await?);
        let _ = BatteryEfficiency::try_estimate(
            battery_logs.find(Interval::try_since(self.estimation.duration())?).await?,
        )
        .await?;
        Ok(())
    }
}

#[derive(Parser)]
pub struct BurrowFoxEssArgs {
    #[clap(flatten)]
    fox_ess_api: FoxEssApiArgs,

    #[command(subcommand)]
    command: BurrowFoxEssCommand,
}

impl BurrowFoxEssArgs {
    #[instrument(skip_all)]
    async fn burrow(self) -> Result {
        let fox_ess = foxess::Api::new(self.fox_ess_api.api_key)?;

        match self.command {
            BurrowFoxEssCommand::Schedule => {
                let schedule = fox_ess.get_schedule(&self.fox_ess_api.serial_number).await?;
                info!(schedule.is_enabled, "gotcha");
                println!("{}", build_time_slot_sequence_table(&schedule.groups));
            }
        }

        Ok(())
    }
}

#[derive(Subcommand)]
enum BurrowFoxEssCommand {
    /// Get the schedule.
    Schedule,
}
