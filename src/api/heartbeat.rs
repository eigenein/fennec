use reqwest::Url;

use crate::{api::client, prelude::*};

#[instrument(skip_all)]
pub async fn send(url: Url) -> Result {
    info!(%url, "Sending a heartbeat…");
    client::try_new()?.post(url).send().await?;
    Ok(())
}
