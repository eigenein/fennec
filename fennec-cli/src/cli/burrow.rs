use chrono::TimeDelta;
use clap::{Parser, Subcommand};

use crate::{
    cli::{battery::BatteryPowerLimits, db::DbArgs},
    db::power,
    energy,
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
        }
    }
}

#[derive(Subcommand)]
pub enum BurrowCommand {
    /// Estimate energy balance profile.
    EnergyBalanceProfile(BurrowEnergyBalanceProfileArgs),
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
            energy::Profile::try_estimate(self.power_limits, bucket_time_step, logs).await?;
        db.shutdown().await;
        println!("{profile}");
        Ok(())
    }
}
