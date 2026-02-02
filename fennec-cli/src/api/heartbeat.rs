use std::time::Duration;

use reqwest::Url;

use crate::prelude::*;

pub struct Client {
    inner: Option<reqwest::Client>,
    url: Option<Url>,
}

impl Client {
    pub fn new(url: Option<Url>) -> Self {
        let inner = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .inspect_err(|error| warn!("failed to initialize the heartbeat client: {error:#}"))
            .map(Some)
            .unwrap_or_default();
        Self { inner, url }
    }

    #[instrument(skip_all)]
    pub async fn send(&self) {
        if let Some(client) = &self.inner
            && let Some(url) = &self.url
            && let Err(error) = client.post(url.clone()).send().await
        {
            warn!("failed to send the heartbeat: {error:#}");
        }
    }
}
