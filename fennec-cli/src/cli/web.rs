use std::net::IpAddr;

use clap::Parser;

#[derive(Parser)]
pub struct BindArgs {
    #[clap(long = "bind-address", env = "BIND_ADDRESS", default_value = "::")]
    pub address: IpAddr,

    #[clap(long = "bind-port", env = "BIND_PORT", default_value = "80")]
    pub port: u16,
}
