use chrono::TimeDelta;
use clap::{Parser, Subcommand};

use crate::{
    api::fox_cloud,
    cli::{battery::BatteryPowerLimits, db::DbArgs, fox_cloud::FoxCloudApiArgs},
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
            BurrowCommand::FoxCloud(args) => args.run().await,
        }
    }
}

#[derive(Subcommand)]
pub enum BurrowCommand {
    /// Estimate energy balance profile.
    EnergyBalanceProfile(BurrowEnergyBalanceProfileArgs),

    /// Test Fox Cloud API connectivity.
    FoxCloud(BurrowFoxCloudArgs),
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
pub struct BurrowFoxCloudArgs {
    #[clap(flatten)]
    fox_cloud: FoxCloudApiArgs,

    #[command(subcommand)]
    command: BurrowFoxCloudCommand,
}

impl BurrowFoxCloudArgs {
    #[instrument(skip_all)]
    async fn run(self) -> Result {
        let client = fox_cloud::Client::new(self.fox_cloud.api_key, self.fox_cloud.serial_number)?;

        match self.command {
            BurrowFoxCloudCommand::Schedule => {
                let schedule = client.get_schedule().await?;
                info!(schedule.is_enabled, "gotcha");
                println!("{}", &schedule.groups);
            }
        }

        Ok(())
    }
}

#[derive(Subcommand)]
enum BurrowFoxCloudCommand {
    /// Get the schedule.
    Schedule,
}
