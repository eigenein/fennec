use std::{ops::RangeInclusive, time::Duration};

use chrono::{DateTime, Local};
use reqwest::{
    Client,
    ClientBuilder,
    Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde_with::serde_as;

use crate::{prelude::*, quantity::energy::KilowattHours};

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

    #[instrument(skip_all, name = "Fetching the energy history…", fields(entity_id = entity_id))]
    pub async fn get_energy_history(
        &self,
        entity_id: &str,
        period: &RangeInclusive<DateTime<Local>>,
    ) -> Result<Vec<EnergyState>> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|()| anyhow!("invalid base URL"))?
            .push("history")
            .push("period")
            .push(&period.start().to_rfc3339());
        url.query_pairs_mut()
            .append_pair("filter_entity_id", entity_id)
            .append_pair("end_time", &period.end().to_rfc3339());
        let entities_history: Vec<EnergyHistory> =
            self.client.get(url).send().await?.error_for_status()?.json().await?;
        let entity_history = entities_history
            .into_iter()
            .next()
            .with_context(|| format!("the API returned no data for `{entity_id}`"))?;
        info!("Fetched", len = entity_history.0.len());
        Ok(entity_history.0)
    }
}

#[must_use]
#[serde_as]
#[derive(serde::Deserialize, derive_more::Index, derive_more::IntoIterator)]
struct EnergyHistory(#[serde_as(as = "serde_with::VecSkipError<_>")] pub Vec<EnergyState>);

#[must_use]
#[serde_as]
#[derive(serde::Deserialize)]
pub struct EnergyState {
    #[serde(rename = "last_changed")]
    pub last_changed_at: DateTime<Local>,

    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[serde(rename = "state")]
    pub total_usage: KilowattHours,

    pub attributes: EnergyAttributes,
}

#[derive(serde::Deserialize)]
pub struct EnergyAttributes {
    #[serde(rename = "custom_total_solar_yield")]
    pub total_solar_yield: KilowattHours,
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
                        "entity_id": "sensor.custom_fennec_hourly_total_energy_usage",
                        "state": "40187.582",
                        "attributes": {
                            "state_class": "total",
                            "custom_now": "2025-10-27 14:15:00.458187+01:00",
                            "custom_total_solar_yield": 14651.505,
                            "custom_battery_residual_energy": 5.16,
                            "custom_battery_energy_import": 366.963,
                            "custom_battery_energy_export": 301.973,
                            "unit_of_measurement": "kWh",
                            "icon": "mdi:flash",
                            "friendly_name": "Fennec total energy usage"
                        },
                        "last_changed": "2025-10-27T13:15:00.458479+00:00",
                        "last_updated": "2025-10-27T13:15:00.458479+00:00"
                    },
                    {
                        "entity_id": "sensor.custom_fennec_hourly_total_energy_usage",
                        "state": "40187.582",
                        "attributes": {
                            "state_class": "total",
                            "unit_of_measurement": "kWh",
                            "icon": "mdi:flash",
                            "friendly_name": "Fennec total energy usage"
                        },
                        "last_changed": "2025-10-27T13:15:00.458479+00:00",
                        "last_updated": "2025-10-27T13:15:00.458479+00:00"
                    }
                ]
            ]
        "#;
        let history = serde_json::from_str::<Vec<EnergyHistory>>(RESPONSE)?;
        let total_energy_usage = history.into_iter().next().unwrap();
        assert_eq!(total_energy_usage.0.len(), 1);

        let expected_timestamp = NaiveDate::from_ymd_opt(2025, 10, 27)
            .unwrap()
            .and_hms_micro_opt(14, 15, 0, 458479)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();

        let state = &total_energy_usage[0];
        assert_eq!(state.last_changed_at, expected_timestamp);
        assert_abs_diff_eq!(state.total_usage.0, 40187.582);
        assert_abs_diff_eq!(state.attributes.total_solar_yield.0, 14651.505);

        Ok(())
    }
}
