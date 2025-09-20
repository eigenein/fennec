use reqwest::Url;

use crate::prelude::*;

#[allow(clippy::literal_string_with_formatting_args)]
#[instrument(skip_all, name = "Sending a heartbeat…")]
pub async fn send(url: Url) {
    if let Err(error) = reqwest::Client::new().post(url).send().await {
        warn!("Failed to send the heartbeat: {error:#}");
    }
}
