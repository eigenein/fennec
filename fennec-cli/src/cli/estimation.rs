use clap::Parser;

#[derive(Parser)]
pub struct EstimationArgs {
    #[clap(long, env, default_value = "none")]
    pub weight_mode: WeightMode,
}

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
pub enum WeightMode {
    /// Unweighted linear regression.
    None,

    /// Total energy flow into and out from the battery.
    EnergyFlow,
}
