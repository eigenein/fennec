use clap::Parser;

use crate::api::modbus;

#[must_use]
#[derive(Parser)]
pub struct PvUrls {
    /// Modbus URL for the total PV yield in watt-hours.
    #[clap(long = "pv-total-yield-url", env = "PV_TOTAL_YIELD_URL")]
    pub total_yield: modbus::ParsedUrl,
}
