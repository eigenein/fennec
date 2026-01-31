use std::time::Duration;

use reqwest::{Client, Url};

use crate::prelude::*;

#[instrument(skip_all)]
pub async fn send(url: Url) -> Result {
    info!("sending a heartbeat…");
    Client::builder().timeout(Duration::from_secs(3)).build()?.post(url).send().await?;
    Ok(())
}
