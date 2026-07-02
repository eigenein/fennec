use std::time::Duration;

use crate::prelude::*;

pub struct Client(Option<(reqwest::Url, reqwest::Client)>);

impl Client {
    #[instrument(skip_all)]
    pub fn new(url: Option<reqwest::Url>) -> Result<Self> {
        let inner = match url {
            Some(url) => {
                Some((url, reqwest::Client::builder().timeout(Duration::from_secs(1)).build()?))
            }
            None => None,
        };
        Ok(Self(inner))
    }

    #[instrument(skip_all)]
    pub async fn send(&self) {
        if let Some((url, client)) = &self.0
            && let Err(error) = Self::inner_send(url, client).await
        {
            warn!("failed heartbeat: {error:#}");
        }
    }

    async fn inner_send(url: &reqwest::Url, client: &reqwest::Client) -> Result {
        client.post(url.clone()).send().await?.error_for_status()?;
        Ok(())
    }
}
