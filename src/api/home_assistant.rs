use std::{ops::RangeInclusive, time::Duration};

use chrono::{DateTime, Local};
use http::Uri;
use serde_with::serde_as;
use ureq::{Agent, tls::TlsConfig};

use crate::{prelude::*, quantity::energy::KilowattHours};

pub struct Api {
    client: Agent,
    base_uri: Uri,
    authorization: String,
}

impl Api {
    pub fn new(access_token: &str, base_uri: Uri) -> Self {
        let authorization = format!("Bearer {access_token}");
        let tls_config = TlsConfig::builder().disable_verification(true).build(); // FIXME
        let client = Agent::config_builder()
            .tls_config(tls_config)
            .timeout_global(Some(Duration::from_secs(10)))
            .build()
            .into();
        Self { client, base_uri, authorization }
    }

    #[instrument(skip_all)]
    pub fn get_energy_history(
        &self,
        entity_id: &str,
        period: &RangeInclusive<DateTime<Local>>,
    ) -> Result<Vec<EnergyState>> {
        info!(entity_id, since = ?period.start(), until = ?period.end(), "Fetching…");
        let entities_history: Vec<EnergyHistory> = self
            .client
            .get(format!("{}/history/period/{}", self.base_uri, period.start().to_rfc3339()))
            .query("filter_entity_id", entity_id)
            .query("end_time", period.end().to_rfc3339())
            .header("Authorization", &self.authorization)
            .call()?
            .body_mut()
            .read_json()?;
        let entity_history = entities_history
            .into_iter()
            .next()
            .with_context(|| format!("the API returned no data for `{entity_id}`"))?;
        info!(len = entity_history.0.len(), "Fetched");
        Ok(entity_history.0)
    }
}

#[must_use]
#[serde_as]
#[derive(serde::Deserialize, derive_more::Index, derive_more::IntoIterator)]
struct EnergyHistory(#[serde_as(as = "serde_with::VecSkipError<_>")] pub Vec<EnergyState>);

#[must_use]
#[serde_as]
#[derive(Copy, Clone, serde::Deserialize)]
pub struct EnergyState {
    #[serde(rename = "last_changed")]
    pub last_changed_at: DateTime<Local>,

    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[serde(rename = "state")]
    pub net_consumption: KilowattHours,

    pub attributes: EnergyAttributes,
}

#[derive(Copy, Clone, derive_more::Add, derive_more::Sub, derive_more::Sum, serde::Deserialize)]
pub struct EnergyAttributes {
    #[serde(rename = "custom_battery_energy_import")]
    pub import: KilowattHours,

    #[serde(rename = "custom_battery_energy_export")]
    pub export: KilowattHours,

    #[serde(rename = "custom_battery_residual_energy")]
    pub residual_energy: KilowattHours,
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
                        "state": "25902.706",
                        "attributes": {
                            "state_class": "total",
                            "custom_now": "2025-11-19 12:55:00.063033+01:00",
                            "custom_battery_residual_energy": 3.86,
                            "custom_battery_energy_import": 473.809,
                            "custom_battery_energy_export": 388.752,
                            "unit_of_measurement": "kWh",
                            "icon": "mdi:flash",
                            "friendly_name": "Fennec total energy usage"
                        },
                        "last_changed": "2025-11-19T11:55:00.063700+00:00",
                        "last_updated": "2025-11-19T11:55:00.063700+00:00"
                    },
                    {
                        "entity_id": "sensor.custom_fennec_hourly_total_energy_usage",
                        "state": "invalid",
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

        let expected_timestamp = NaiveDate::from_ymd_opt(2025, 11, 19)
            .unwrap()
            .and_hms_micro_opt(12, 55, 0, 63700)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();

        let state = &total_energy_usage[0];
        assert_eq!(state.last_changed_at, expected_timestamp);
        assert_abs_diff_eq!(state.net_consumption.0.0, 25902.706);
        assert_abs_diff_eq!(state.attributes.import.0.0, 473.809);
        assert_abs_diff_eq!(state.attributes.export.0.0, 388.752);
        assert_abs_diff_eq!(state.attributes.residual_energy.0.0, 3.86);

        Ok(())
    }
}
