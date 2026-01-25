use std::time::Duration;

use reqwest::{Client, Url};

use crate::prelude::*;

#[instrument(skip_all)]
pub async fn send(url: Url) {
    if let Err(error) = send_fallible(url).await {
        warn!("failed to send the heartbeat: {error:#}");
    }
}

async fn send_fallible(url: Url) -> Result {
    info!(%url, "sending a heartbeat…");
    Client::builder().timeout(Duration::from_secs(10)).build()?.post(url).send().await?;
    Ok(())
}
