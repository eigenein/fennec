use chrono::{DateTime, Local};
use serde_with::serde_as;

#[must_use]
#[derive(serde::Deserialize, derive_more::IntoIterator)]
pub struct EntitiesHistory<A>(
    #[serde(bound(deserialize = "A: serde::de::DeserializeOwned"))] pub Vec<EntityHistory<A>>,
);

#[must_use]
#[serde_as]
#[derive(serde::Deserialize, derive_more::Index, derive_more::IntoIterator)]
pub struct EntityHistory<A>(
    #[serde_as(as = "serde_with::VecSkipError<_>")]
    #[serde(bound(deserialize = "A: serde::de::DeserializeOwned"))]
    pub Vec<State<A>>,
);

#[must_use]
#[serde_as]
#[derive(serde::Deserialize)]
pub struct State<A> {
    #[serde(rename = "last_changed")]
    pub last_changed_at: DateTime<Local>,

    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[serde(rename = "state")]
    pub value: f64,

    pub attributes: A,
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use chrono::NaiveDate;

    use super::*;
    use crate::{
        api::home_assistant::battery::BatteryStateAttributes,
        prelude::*,
        quantity::energy::KilowattHours,
    };

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
                            "custom_battery_residual_energy": 5.62,
                            "custom_battery_energy_import": 198.52,
                            "custom_battery_energy_export": 162.646,
                            "unit_of_measurement": "kWh",
                            "icon": "mdi:flash",
                            "friendly_name": "Fennec sensor"
                        },
                        "last_changed": "2025-10-02T15:17:17.713307+00:00",
                        "last_updated": "2025-10-02T15:17:17.713307+00:00"
                    },
                    {
                        "entity_id": "sensor.custom_fennec_sensor",
                        "state": "39790.578",
                        "attributes": {
                            "state_class": "total",
                            "custom_battery_residual_energy": 5.62,
                            "custom_battery_energy_import": 198.52,
                            "custom_battery_energy_export": 162.646,
                            "unit_of_measurement": "kWh",
                            "icon": "mdi:flash",
                            "friendly_name": "Fennec sensor"
                        },
                        "last_changed": "2025-10-02T15:17:17.713307+00:00",
                        "last_updated": "2025-10-02T15:17:17.713307+00:00"
                    }
                ]
            ]
        "#;
        let history = serde_json::from_str::<EntitiesHistory<BatteryStateAttributes<KilowattHours>>>(
            RESPONSE,
        )?;
        let total_energy_usage = history.into_iter().next().unwrap();
        assert_eq!(total_energy_usage.0.len(), 1);
        let state = &total_energy_usage[0];
        assert_eq!(
            state.last_changed_at,
            NaiveDate::from_ymd_opt(2025, 10, 2)
                .unwrap()
                .and_hms_micro_opt(17, 17, 17, 713307)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap()
        );
        assert_abs_diff_eq!(state.value, 39790.578);
        assert_abs_diff_eq!(state.attributes.residual_energy.0, 5.62);
        assert_abs_diff_eq!(state.attributes.total_import.0, 198.52);
        assert_abs_diff_eq!(state.attributes.total_export.0, 162.646);
        Ok(())
    }
}
