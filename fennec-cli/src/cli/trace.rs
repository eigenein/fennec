use clap::Parser;

use crate::{
    energy::{ProfileManager, ProfileState},
    prelude::*,
};

#[derive(Parser)]
pub struct TraceArgs {}

impl TraceArgs {
    pub async fn run(self) -> Result {
        let balance = ProfileState::read_from(ProfileManager::PATH).await?.get_average();
        info!(
            grid_import = ?balance.grid.import,
            grid_export = ?balance.grid.export,
            battery_import = ?balance.battery.import,
            battery_export = ?balance.battery.export,
        );
        Ok(())
    }
}
