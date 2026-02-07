use chrono::{DateTime, Local, TimeDelta};
use clap::Parser;

#[derive(Parser)]
pub struct EstimationArgs {
    /// Measurement window duration to select from the readings when estimating battery efficiency.
    #[clap(
        long = "battery-estimation-interval",
        env = "BATTERY_ESTIMATION_INTERVAL",
        default_value = "14d"
    )]
    duration: humantime::Duration,

    #[clap(long, env, default_value = "energy-flow")]
    pub weight_mode: WeightMode,
}

impl EstimationArgs {
    pub fn since(&self) -> DateTime<Local> {
        Local::now() - TimeDelta::from_std(self.duration.into()).unwrap()
    }
}

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
pub enum WeightMode {
    /// Unweighted linear regression.
    None,

    /// Total energy flow into and out from the battery.
    EnergyFlow,
}
