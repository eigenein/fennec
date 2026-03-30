use clap::Parser;

use crate::{api::fox_cloud, prelude::*};

#[derive(Parser)]
pub struct FoxCloudConnectionArgs {
    #[clap(flatten)]
    api: FoxCloudApiArgs,

    /// Do not push schedules to Fox Cloud – only perform dry runs.
    #[clap(long, env = "SCOUT")]
    scout: bool,
}

impl FoxCloudConnectionArgs {
    pub fn client(self) -> Result<Option<fox_cloud::Client>> {
        (!self.scout).then(|| self.api.client()).transpose()
    }
}

#[derive(Parser)]
pub struct FoxCloudApiArgs {
    #[clap(long = "api-key", env = "FOX_ESS_API_KEY")]
    pub api_key: String,

    #[clap(long, alias = "serial", env = "FOX_ESS_SERIAL_NUMBER")]
    pub serial_number: String,
}

impl FoxCloudApiArgs {
    pub fn client(self) -> Result<fox_cloud::Client> {
        fox_cloud::Client::new(self.api_key, self.serial_number)
    }
}
