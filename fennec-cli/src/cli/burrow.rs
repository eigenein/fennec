use clap::{Parser, Subcommand};

use crate::{
    api::foxcloud,
    cli::{
        battery::BatteryPowerLimits,
        db::DbArgs,
        estimation::EstimationArgs,
        foxess::FoxEssApiArgs,
    },
    db::{battery, power},
    prelude::*,
    statistics::{FlowStatistics, battery::BatteryEfficiency},
};

#[derive(Parser)]
pub struct BurrowArgs {
    #[command(subcommand)]
    command: BurrowCommand,
}

impl BurrowArgs {
    pub async fn run(self) -> Result {
        match self.command {
            BurrowCommand::Battery(args) => args.run().await,
            BurrowCommand::Consumption(args) => args.run().await,
            BurrowCommand::FoxEss(args) => args.run().await,
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
    async fn run(self) -> Result {
        let db = self.db.connect().await?;
        let logs = db.measurements::<battery::Measurement>().await?;
        let _ = BatteryEfficiency::try_estimate(logs, self.estimation.weight_mode).await?;
        db.shutdown().await;
        Ok(())
    }
}

#[derive(Parser)]
pub struct BurrowConsumptionArgs {
    #[clap(flatten)]
    db: DbArgs,

    #[clap(flatten)]
    estimation: EstimationArgs,

    #[clap(flatten)]
    power_limits: BatteryPowerLimits,
}

impl BurrowConsumptionArgs {
    async fn run(self) -> Result {
        let db = self.db.connect().await?;
        let logs = db.measurements::<power::Measurement>().await?;
        let statistics = FlowStatistics::try_estimate(self.power_limits, logs).await?;
        db.shutdown().await;
        println!("{statistics}");
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
    async fn run(self) -> Result {
        let fox_ess = foxcloud::Api::new(self.fox_ess_api.api_key)?;

        match self.command {
            BurrowFoxEssCommand::Schedule => {
                let schedule = fox_ess.get_schedule(&self.fox_ess_api.serial_number).await?;
                info!(schedule.is_enabled, "gotcha");
                println!("{}", &schedule.groups);
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
