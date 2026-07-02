use std::time::Duration;

use http::{HeaderMap, HeaderValue, header};
use serde::Serialize;

use crate::prelude::*;

/// Client for a single state in Home Assistant.
pub struct StateClient(Option<(reqwest::Client, reqwest::Url)>);

impl StateClient {
    #[instrument(skip_all)]
    pub fn new(url: Option<reqwest::Url>) -> Result<Self> {
        let Some(mut url) = url else { return Ok(Self(None)) };

        let bearer_token = url.fragment().context("URL fragment must contain the bearer token")?;
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, format!("Bearer {bearer_token}").try_into()?);
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
        url.set_fragment(None);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(1))
            .build()?;

        Ok(Self(Some((client, url))))
    }

    pub async fn post<T: Serialize>(&self, value: &T) {
        if let Some((client, url)) = &self.0
            && let Err(error) = Self::inner_post(client, url, value).await
        {
            warn!("failed to update the state: {error:#}");
        }
    }

    async fn inner_post<T: Serialize>(
        client: &reqwest::Client,
        url: &reqwest::Url,
        value: &T,
    ) -> Result {
        let state = State { value };
        client.post(url.clone()).json(&state).send().await?.error_for_status()?;
        Ok(())
    }
}

#[derive(Serialize)]
struct State<T> {
    #[serde(rename = "state")]
    value: T,
}
