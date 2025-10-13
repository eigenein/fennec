pub mod battery;
pub mod energy;
pub mod history;

use std::{
    fmt::Display,
    iter::Sum,
    ops::{Add, Div, Mul, RangeInclusive, Sub},
    str::FromStr,
    time::Duration,
};

use chrono::{DateTime, Local, TimeDelta};
use reqwest::{
    Client,
    ClientBuilder,
    Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::de::{DeserializeOwned, IgnoredAny};

use crate::{
    api::home_assistant::history::{EntitiesHistory, EntityHistory},
    core::series::{AverageHourly, Differentiate, Resample, resample_on_time_delta},
    prelude::*,
};

pub struct Api {
    client: Client,
    base_url: Url,
}

impl Api {
    pub fn try_new(access_token: &str, base_url: Url) -> Result<Self> {
        let headers = HeaderMap::from_iter([(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&format!("Bearer {access_token}"))?,
        )]);
        let client = ClientBuilder::new()
            .default_headers(headers)
            .danger_accept_invalid_certs(true) // FIXME
            .danger_accept_invalid_hostnames(true) // FIXME
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self { client, base_url })
    }

    #[instrument(skip_all, name = "Fetching the entity state changes…", fields(entity_id = entity_id))]
    pub async fn get_history<V, A>(
        &self,
        entity_id: &str,
        period: &RangeInclusive<DateTime<Local>>,
    ) -> Result<EntityHistory<V, A>>
    where
        A: DeserializeOwned,
        V: FromStr + DeserializeOwned,
        <V as FromStr>::Err: Display,
    {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|()| anyhow!("invalid base URL"))?
            .push("history")
            .push("period")
            .push(&period.start().to_rfc3339());
        url.query_pairs_mut()
            .append_pair("filter_entity_id", entity_id)
            .append_pair("end_time", &period.end().to_rfc3339());
        let entities_history: EntitiesHistory<V, A> =
            self.client.get(url).send().await?.error_for_status()?.json().await?;
        let entity_history = entities_history
            .into_iter()
            .next()
            .with_context(|| format!("the API returned no data for `{entity_id}`"))?;
        info!("Fetched", len = entity_history.0.len());
        Ok(entity_history)
    }

    pub async fn get_average_hourly_history<V>(
        &self,
        entity_id: &str,
        period: &RangeInclusive<DateTime<Local>>,
    ) -> Result<[Option<<<V as Div<TimeDelta>>::Output as Div<f64>>::Output>; 24]>
    where
        <V as Div<TimeDelta>>::Output: Copy
            + Mul<TimeDelta, Output = V>
            + Div<f64, Output = <V as Div<TimeDelta>>::Output>
            + Sum,
        <V as FromStr>::Err: Display,
        V: Copy
            + Default
            + Add<V, Output = V>
            + Sub<V, Output = V>
            + Div<TimeDelta>
            + FromStr
            + DeserializeOwned,
    {
        Ok(self
            .get_history::<V, IgnoredAny>(entity_id, period)
            .await?
            .into_iter()
            .map(|state| (state.last_changed_at, state.value))
            .resample(resample_on_time_delta(TimeDelta::hours(1)))
            .deltas()
            .average_hourly())
    }
}
