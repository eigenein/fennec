use chrono::{DateTime, Local};
use serde_with::serde_as;

#[must_use]
#[derive(serde::Deserialize, derive_more::IntoIterator)]
#[serde(bound(
    deserialize = "V: std::str::FromStr + serde::de::DeserializeOwned, <V as std::str::FromStr>::Err: std::fmt::Display"
))]
pub struct EntitiesHistory<V>(pub Vec<EntityHistory<V>>);

#[must_use]
#[serde_as]
#[derive(serde::Deserialize, derive_more::Index, derive_more::IntoIterator)]
#[serde(bound(
    deserialize = "V: std::str::FromStr + serde::de::DeserializeOwned, <V as std::str::FromStr>::Err: std::fmt::Display"
))]
pub struct EntityHistory<V>(#[serde_as(as = "serde_with::VecSkipError<_>")] pub Vec<State<V>>);

#[must_use]
#[serde_as]
#[derive(Copy, Clone, serde::Deserialize)]
#[serde(bound(
    deserialize = "V: std::str::FromStr + serde::de::DeserializeOwned, <V as std::str::FromStr>::Err: std::fmt::Display",
))]
pub struct State<V> {
    #[serde(rename = "last_changed")]
    pub last_changed_at: DateTime<Local>,

    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[serde(rename = "state")]
    pub value: V,
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use chrono::NaiveDate;

    use super::*;
    use crate::{prelude::*, quantity::energy::KilowattHours};

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
        let history = serde_json::from_str::<EntitiesHistory<KilowattHours>>(RESPONSE)?;
        let total_energy_usage = history.into_iter().next().unwrap();
        assert_eq!(total_energy_usage.0.len(), 2);

        let expected_timestamp = NaiveDate::from_ymd_opt(2025, 10, 27)
            .unwrap()
            .and_hms_micro_opt(14, 15, 0, 458479)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();

        let state = &total_energy_usage[0];
        assert_eq!(state.last_changed_at, expected_timestamp);
        assert_abs_diff_eq!(state.value.0, 40187.582);

        Ok(())
    }
}
