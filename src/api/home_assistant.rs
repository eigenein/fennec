use chrono::{DateTime, Local};
use reqwest::{
    Client,
    ClientBuilder,
    Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
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

    #[instrument(skip_all, name = "Fetching state…", fields(entity_id = entity_id))]
    pub async fn get_state(&self, entity_id: &str) -> Result<State> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("invalid base URL"))?
            .push("states")
            .push(entity_id);
        Ok(self.client.get(url).send().await?.error_for_status()?.json().await?)
    }
}

#[must_use]
#[serde_as]
#[derive(serde::Deserialize)]
pub struct State {
    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[serde(rename = "state")]
    pub value: f64,

    #[serde(rename = "last_reported")]
    pub last_reported_at: DateTime<Local>,

    #[allow(dead_code)]
    pub attributes: StateAttributes,
}

#[derive(serde::Deserialize)]
pub struct StateAttributes {
    #[allow(dead_code)]
    #[serde(rename = "state_class")]
    class: StateClass,
}

/// [State classes][1].
///
/// [1]: https://developers.home-assistant.io/docs/core/entity/sensor/#available-state-classes
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Deserialize)]
pub enum StateClass {
    /// The state represents a total amount that can both increase and decrease, e.g. a net energy meter.
    #[serde(rename = "total")]
    Total,

    /// Similar to [`StateClass::Total`], with the restriction
    /// that the state represents a monotonically increasing positive total
    /// which periodically restarts counting from 0.
    #[serde(rename = "total_increasing")]
    TotalIncreasing,

    #[serde(other)]
    Other,
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn test_deserialize_response_ok() -> Result {
        // language=JSON
        const RESPONSE: &str = r#"
            {
                "entity_id": "sensor.custom_total_energy_usage",
                "state": "39615.719",
                "attributes": {
                    "state_class": "total",
                    "unit_of_measurement": "kWh",
                    "icon": "mdi:flash",
                    "friendly_name": "Total energy usage"
                },
                "last_changed": "2025-09-20T16:51:49.339572+00:00",
                "last_reported": "2025-09-20T16:51:49.339572+00:00",
                "last_updated": "2025-09-20T16:51:49.339572+00:00",
                "context": {
                    "id": "01K5M0KZESFSEPEWVWQCHD8VF5",
                    "parent_id": null,
                    "user_id": null
                }
            }
        "#;
        let state = serde_json::from_str::<State>(RESPONSE)?;
        assert_eq!(state.value, 39615.719);
        assert_eq!(state.attributes.class, StateClass::Total);
        assert_eq!(state.last_reported_at, Local.timestamp_micros(1_758_387_109_339_572).unwrap());
        Ok(())
    }
}
