use clap::Parser;

use crate::{energy::ExponentialProfile, prelude::*};

#[derive(Parser)]
pub struct TraceArgs {}

impl TraceArgs {
    pub async fn run(self) -> Result {
        let balance = ExponentialProfile::read().await?.get_average();
        info!(
            grid_import = ?balance.grid.import,
            grid_export = ?balance.grid.export,
            battery_import = ?balance.battery.import,
            battery_export = ?balance.battery.export,
        );
        Ok(())
    }
}
