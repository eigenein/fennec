use clap::{Parser, Subcommand};
use comfy_table::{Cell, Table, modifiers, presets};

use crate::{
    api::foxess,
    cli::{db::DbArgs, estimation::EstimationArgs, foxess::FoxEssApiArgs},
    core::interval::Interval,
    db::{battery::BatteryLog, consumption::ConsumptionLog},
    prelude::*,
    statistics::{battery::BatteryEfficiency, consumption::ConsumptionStatistics},
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
            BurrowCommand::Battery(args) => args.burrow().await,
            BurrowCommand::Consumption(args) => args.burrow().await,
            BurrowCommand::FoxEss(args) => args.burrow().await,
        }
    }
}

#[derive(Subcommand)]
pub enum BurrowCommand {
    /// Estimate battery efficiency parameters.
    Battery(BurrowBatteryArgs),

    /// Estimate net consumption profile.
    Consumption(BurrowConsumptionArgs),

    /// Test FoxESS Cloud API connectivity.
    FoxEss(BurrowFoxEssArgs),
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
        let db = self.db.connect().await?;
        let logs =
            db.find_logs::<BatteryLog>(Interval::try_since(self.estimation.duration())?).await?;
        let _ = BatteryEfficiency::try_estimate(logs).await?;
        Ok(())
    }
}

#[derive(Parser)]
pub struct BurrowConsumptionArgs {
    #[clap(flatten)]
    db: DbArgs,

    #[clap(flatten)]
    estimation: EstimationArgs,
}

impl BurrowConsumptionArgs {
    async fn burrow(self) -> Result {
        let db = self.db.connect().await?;
        let logs = db
            .find_logs::<ConsumptionLog>(Interval::try_since(self.estimation.duration())?)
            .await?;
        let statistics = ConsumptionStatistics::try_estimate(logs).await?;
        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS);
        table.enforce_styling();
        table.set_header(vec!["Hour", "Power"]);
        for (hour, power) in statistics.hourly.iter().enumerate() {
            table.add_row(vec![
                Cell::new(hour),
                power.map(Cell::new).unwrap_or_else(|| Cell::new("n/a")),
            ]);
        }
        println!("{table}");
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
