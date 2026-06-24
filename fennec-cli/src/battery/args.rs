use crate::{battery, battery::WorkingMode, quantity::price::KilowattHourPrice};

#[derive(clap::Args)]
#[group(id = "battery")]
pub struct Args {
    #[clap(
        long = "battery-working-modes",
        env = "WORKING_MODES",
        value_delimiter = ',',
        num_args = 1..,
        default_value = "harness,compensate,charge,self-use",
    )]
    pub working_modes: Vec<WorkingMode>,

    #[clap(flatten)]
    pub power_limits: battery::PowerLimits,

    /// Battery health costs lost to the cycling, in ¤/kWh.
    #[clap(
        long = "battery-degradation-cost",
        env = "BATTERY_DEGRADATION_COST",
        default_value = "0.01"
    )]
    pub degradation_cost: KilowattHourPrice,
}
