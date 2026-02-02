use std::time::Duration;

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
}

impl EstimationArgs {
    pub fn duration(&self) -> Duration {
        self.duration.into()
    }
}
