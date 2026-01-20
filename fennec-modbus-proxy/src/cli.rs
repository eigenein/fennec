use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, propagate_version = true)]
pub struct Args {
    #[clap(long, default_value = "0.0.0.0:80", env = "BIND_ADDRESS")]
    pub bind_address: String,

    #[clap(flatten)]
    pub battery: BatteryArgs,
}

#[derive(Parser)]
pub struct BatteryArgs {
    #[clap(flatten)]
    pub connection: BatteryConnectionArgs,

    #[clap(flatten)]
    pub registers: BatteryRegisterArgs,
}

#[derive(Parser)]
pub struct BatteryConnectionArgs {
    #[clap(long, env = "BATTERY_ADDRESS")]
    pub address: String,

    #[clap(long, default_value = "1", env = "SLAVE_ID")]
    pub slave_id: u8,
}

#[derive(Copy, Clone, Parser)]
pub struct BatteryRegisterArgs {
    #[clap(long, default_value = "39424", env = "SOC_REGISTER")]
    pub state_of_charge: u16,

    #[clap(long, default_value = "37624", env = "SOH_REGISTER")]
    pub state_of_health: u16,

    #[clap(long, default_value = "37635", env = "DESIGN_ENERGY_REGISTER")]
    pub design_energy: u16,
}
