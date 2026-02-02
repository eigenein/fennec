use std::time::Duration;

use clap::Parser;
use reqwest::{Client, Url};

use crate::prelude::*;

#[derive(Parser)]
pub struct HeartbeatArgs {
    #[clap(long = "heartbeat-url", env = "HEARTBEAT_URL")]
    pub url: Option<Url>,
}

impl HeartbeatArgs {
    pub async fn send(&self) {
        if let Some(url) = &self.url
            && let Err(error) = Self::send_fallible(url.clone()).await
        {
            warn!("failed to send the heartbeat: {error:#}");
        }
    }

    #[instrument(skip_all)]
    async fn send_fallible(url: Url) -> Result {
        info!("sending a heartbeat…");
        Client::builder().timeout(Duration::from_secs(3)).build()?.post(url).send().await?;
        Ok(())
    }
}
