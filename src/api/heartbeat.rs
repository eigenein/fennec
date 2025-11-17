use std::time::Duration;

use reqwest::Url;

use crate::prelude::*;

#[instrument(skip_all)]
pub async fn send(url: Url) {
    info!(%url, "Sending a heartbeat…");
    if let Err(error) =
        reqwest::Client::new().post(url).timeout(Duration::from_secs(10)).send().await
    {
        warn!("Failed to send the heartbeat: {error:#}");
    }
}
