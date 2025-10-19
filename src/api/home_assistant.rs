pub mod energy;
pub mod history;

use std::{fmt::Display, ops::RangeInclusive, str::FromStr, time::Duration};

use chrono::{DateTime, Local, TimeDelta};
use reqwest::{
    Client,
    ClientBuilder,
    Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::de::DeserializeOwned;

use crate::{
    api::home_assistant::history::{EntitiesHistory, EntityHistory},
    core::series::{AverageHourly, Differentiate, Resample},
    prelude::*,
    quantity::{energy::KilowattHours, power::Kilowatts},
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
    pub async fn get_history<V>(
        &self,
        entity_id: &str,
        period: &RangeInclusive<DateTime<Local>>,
    ) -> Result<EntityHistory<V>>
    where
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
        let entities_history: EntitiesHistory<V> =
            self.client.get(url).send().await?.error_for_status()?.json().await?;
        let entity_history = entities_history
            .into_iter()
            .next()
            .with_context(|| format!("the API returned no data for `{entity_id}`"))?;
        info!("Fetched", len = entity_history.0.len());
        Ok(entity_history)
    }

    pub async fn get_average_hourly_power(
        &self,
        entity_id: &str,
        period: &RangeInclusive<DateTime<Local>>,
    ) -> Result<([Option<Kilowatts>; 24], Option<Kilowatts>)> {
        const ONE_HOUR: TimeDelta = TimeDelta::hours(1);

        let mut from_point = None;
        let mut to_point = None;
        let hourly_power = self
            .get_history::<KilowattHours>(entity_id, period)
            .await?
            .into_iter()
            .inspect(|state| {
                if state.last_changed_at >= *period.end() - ONE_HOUR {
                    let point = Some((state.last_changed_at, state.value));
                    if from_point.is_none() {
                        from_point = point;
                    }
                    to_point = point;
                }
            })
            .map(|state| (state.last_changed_at, state.value))
            .resample_by_interval(ONE_HOUR)
            .deltas()
            .map(|(timestamp, (dt, dv))| (timestamp, dv / dt))
            .average_hourly();

        let last_hour_power = from_point.zip(to_point).and_then(
            |((from_timestamp, from_value), (to_timestamp, to_value))| {
                let power = (to_value - from_value) / (to_timestamp - from_timestamp);
                power.0.is_finite().then_some(power)
            },
        );
        Ok((hourly_power, last_hour_power))
    }
}
