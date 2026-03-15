use clap::Parser;
use url::Url;

#[derive(Parser)]
pub struct HeartbeatArgs {
    #[clap(long = "heartbeat-url", env = "HEARTBEAT_URL")]
    pub url: Option<Url>,
}
