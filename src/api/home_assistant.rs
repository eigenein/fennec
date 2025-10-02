use chrono::{DateTime, Local};
use reqwest::{
    Client,
    ClientBuilder,
    Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::Deserialize;
use serde_with::serde_as;

use crate::{
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
            .build()?;
        Ok(Self { client, base_url })
    }

    #[instrument(skip_all, name = "Fetching the entity state changes…", fields(entity_id = entity_id
    ))]
    pub async fn get_history(
        &self,
        entity_id: &str,
        from: DateTime<Local>,
        until: DateTime<Local>,
    ) -> Result<EntityHistory> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|()| anyhow!("invalid base URL"))?
            .push("history")
            .push("period")
            .push(&from.to_rfc3339());
        url.query_pairs_mut()
            .append_pair("filter_entity_id", entity_id)
            .append_pair("end_time", &until.to_rfc3339());
        let entities_history: EntitiesHistory =
            self.client.get(url).send().await?.error_for_status()?.json().await?;
        let entity_history = entities_history
            .into_iter()
            .next()
            .with_context(|| format!("the API returned no data for `{entity_id}`"))?;
        info!("Fetched", len = entity_history.0.len());
        Ok(entity_history)
    }
}

#[must_use]
#[derive(Deserialize, derive_more::IntoIterator)]
struct EntitiesHistory(pub Vec<EntityHistory>);

#[must_use]
#[serde_as]
#[derive(Deserialize, derive_more::Index, derive_more::IntoIterator)]
pub struct EntityHistory(#[serde_as(as = "serde_with::VecSkipError<_>")] pub Vec<State>);

#[must_use]
#[serde_as]
#[derive(serde::Deserialize)]
pub struct State {
    #[serde(rename = "last_changed")]
    pub last_changed_at: DateTime<Local>,

    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[serde(rename = "state")]
    pub value: f64,

    pub attributes: StateAttributes,
}

#[must_use]
#[derive(serde::Deserialize)]
pub struct StateAttributes {
    #[serde(rename = "custom_battery_residual_energy")]
    pub battery_residual_energy: KilowattHours,

    #[serde(rename = "custom_battery_net_energy_usage")]
    pub battery_net_energy_usage: Kilowatts,
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn test_deserialize_entities_history_ok() -> Result {
        // language=JSON
        const RESPONSE: &str = r#"
            [
                [
                     {
                        "entity_id": "sensor.custom_fennec_sensor",
                        "state": "unavailable",
                        "attributes": {
                            "state_class": "total",
                            "custom_battery_residual_energy": 5.35,
                            "custom_battery_net_energy_usage": 35.61700000000002,
                            "unit_of_measurement": "kWh",
                            "icon": "mdi:flash",
                            "friendly_name": "Fennec sensor"
                        },
                        "last_changed": "2025-10-02T14:47:11.640927+00:00",
                        "last_updated": "2025-10-02T14:47:11.640927+00:00"
                    },
                     {
                        "entity_id": "sensor.custom_fennec_sensor",
                        "state": "39790.284",
                        "attributes": {
                            "state_class": "total",
                            "custom_battery_residual_energy": 5.35,
                            "custom_battery_net_energy_usage": 35.617,
                            "unit_of_measurement": "kWh",
                            "icon": "mdi:flash",
                            "friendly_name": "Fennec sensor"
                        },
                        "last_changed": "2025-10-02T14:47:11.640927+00:00",
                        "last_updated": "2025-10-02T14:47:11.640927+00:00"
                    }
                ]
            ]
        "#;
        let history = serde_json::from_str::<EntitiesHistory>(RESPONSE)?;
        let total_energy_usage = history.into_iter().next().unwrap();
        assert_eq!(total_energy_usage.0.len(), 1);
        let state = &total_energy_usage[0];
        assert_eq!(
            state.last_changed_at,
            NaiveDate::from_ymd_opt(2025, 10, 2)
                .unwrap()
                .and_hms_micro_opt(16, 47, 11, 640927)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap()
        );
        assert_abs_diff_eq!(state.value, 39790.284);
        assert_abs_diff_eq!(state.attributes.battery_net_energy_usage.0, 35.617);
        assert_abs_diff_eq!(state.attributes.battery_residual_energy.0, 5.35);
        Ok(())
    }
}
