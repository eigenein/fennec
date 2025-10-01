use chrono::{DateTime, Local};
use reqwest::{
    Client,
    ClientBuilder,
    Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::Deserialize;
use serde_with::serde_as;

use crate::prelude::*;

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
            .build()?;
        Ok(Self { client, base_url })
    }

    #[instrument(skip_all, name = "Fetching the entity state changes…", fields(entity_id = entity_id))]
    pub async fn get_history(
        &self,
        entity_id: &str,
        from: DateTime<Local>,
        until: DateTime<Local>,
    ) -> Result<EntitiesHistory> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|()| anyhow!("invalid base URL"))?
            .push("history")
            .push("period")
            .push(&from.to_rfc3339());
        url.query_pairs_mut()
            .append_pair("filter_entity_id", entity_id)
            .append_pair("end_time", &until.to_rfc3339())
            .append_pair("no_attributes", "true");
        let entities_history: EntitiesHistory =
            self.client.get(url).send().await?.error_for_status()?.json().await?;
        info!("Fetched", n_entities = entities_history.0.len());
        Ok(entities_history)
    }
}

#[must_use]
#[derive(Deserialize, derive_more::Index, derive_more::IntoIterator)]
pub struct EntitiesHistory(pub Vec<EntityHistory>);

#[must_use]
#[serde_as]
#[derive(Deserialize, derive_more::Index, derive_more::IntoIterator)]
pub struct EntityHistory(#[serde_as(as = "serde_with::VecSkipError<_>")] pub Vec<State>);

#[must_use]
#[serde_as]
#[derive(serde::Deserialize)]
pub struct State {
    #[serde(rename = "last_updated")]
    pub last_updated_at: DateTime<Local>,

    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[serde(rename = "state")]
    pub value: f64,
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn test_deserialize_entities_history_ok() -> Result {
        // language=JSON
        const RESPONSE: &str = r#"
            [
                [
                    {
                        "entity_id": "sensor.custom_total_energy_usage",
                        "state": "invalid",
                        "attributes": {},
                        "last_changed": "2025-10-01T17:08:40.326747+00:00",
                        "last_updated": "2025-10-01T17:08:40.326747+00:00"
                    },
                    {
                        "entity_id": "sensor.custom_total_energy_usage",
                        "state": "39775.108",
                        "attributes": {},
                        "last_changed": "2025-10-01T17:08:40.326747+00:00",
                        "last_updated": "2025-10-01T17:08:40.326747+00:00"
                    }
                ],
                [
                    {
                        "entity_id": "sensor.foxess_residual_energy",
                        "state": "5.65",
                        "attributes": {},
                        "last_changed": "2025-10-01T17:08:21.473819+00:00",
                        "last_updated": "2025-10-01T17:08:21.473819+00:00"
                    }
                ]
            ]
        "#;
        let history = serde_json::from_str::<EntitiesHistory>(RESPONSE)?;
        assert_eq!(history.0.len(), 2);
        let total_energy_usage = &history[0];
        assert_eq!(total_energy_usage.0.len(), 1);
        assert_eq!(total_energy_usage[0].value, 39775.108);
        assert_eq!(
            total_energy_usage[0].last_updated_at,
            Local.timestamp_micros(1_759_338_520_326_747).unwrap()
        );
        Ok(())
    }
}
