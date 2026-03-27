use chrono::TimeDelta;
use clap::{Parser, Subcommand};

use crate::{
    api::foxcloud,
    cli::{battery::BatteryPowerLimits, db::DbArgs, foxcloud::FoxCloudApiArgs},
    db::power,
    energy::BalanceProfile,
    prelude::*,
};

#[derive(Parser)]
pub struct BurrowArgs {
    #[command(subcommand)]
    command: BurrowCommand,
}

impl BurrowArgs {
    pub async fn run(self) -> Result {
        match self.command {
            BurrowCommand::EnergyBalanceProfile(args) => args.run().await,
            BurrowCommand::FoxEss(args) => args.run().await,
        }
    }
}

#[derive(Subcommand)]
pub enum BurrowCommand {
    /// Estimate energy balance profile.
    EnergyBalanceProfile(BurrowEnergyBalanceProfileArgs),

    /// Test FoxESS Cloud API connectivity.
    FoxEss(BurrowFoxEssArgs),
}

#[derive(Parser)]
pub struct BurrowEnergyBalanceProfileArgs {
    #[clap(flatten)]
    db: DbArgs,

    #[clap(flatten)]
    power_limits: BatteryPowerLimits,

    #[clap(long = "bucket-time-step", default_value = "15min")]
    bucket_time_step: humantime::Duration,
}

impl BurrowEnergyBalanceProfileArgs {
    async fn run(self) -> Result {
        let db = self.db.connect().await?;
        let logs = db.measurements::<power::Measurement>().await?;
        let bucket_time_step = TimeDelta::from_std(self.bucket_time_step.into())?;
        let profile =
            BalanceProfile::try_estimate(self.power_limits, bucket_time_step, logs).await?;
        db.shutdown().await;
        println!("{profile}");
        Ok(())
    }
}

#[derive(Parser)]
pub struct BurrowFoxEssArgs {
    #[clap(flatten)]
    fox_ess_api: FoxCloudApiArgs,

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
