use std::time::Duration;

use derive_more::FromStr;

use crate::prelude::*;

pub struct Client {
    inner: reqwest::Client,
    url: reqwest::Url,
}

impl Client {
    #[instrument(skip_all)]
    pub async fn send(&self) {
        if let Err(error) = self.inner.post(self.url.clone()).send().await {
            warn!("failed heartbeat: {error:#}");
        }
    }
}

#[derive(Clone, FromStr)]
pub struct Url(reqwest::Url);

impl Url {
    #[instrument(skip_all, fields(url = %self.0))]
    pub fn client(self) -> Result<Client> {
        let inner = reqwest::Client::builder().timeout(Duration::from_secs(1)).build()?;
        Ok(Client { inner, url: self.0 })
    }
}
