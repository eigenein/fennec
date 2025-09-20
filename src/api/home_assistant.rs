use chrono::{DateTime, Local};
use reqwest::{
    Client,
    ClientBuilder,
    Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde_with::serde_as;

use crate::prelude::*;

#[expect(dead_code)]
pub struct Api {
    client: Client,

    /// Sensor [REST API][1] url for the household total energy usage in kilowatt-hours.
    /// For example: <http://localhost:8123/api/states/sensor.custom_total_energy_usage>.
    ///
    /// The state must have the [`total` or `total_increasing` class][2] and only account for actual household usage,
    /// including the solar panels yield, and excluding exports and the battery consumption and production:
    ///
    /// - Add grid import
    /// - Add solar panels yield
    /// - Add battery export
    /// - Subtract grid export
    /// - Subtract battery import
    ///
    /// # Example template
    ///
    /// ```jinja2
    /// {{
    ///     states('sensor.p1_meter_energy_import') | float
    ///     + states('sensor.sb2_5_1vl_40_555_total_yield') | float
    ///     + states('sensor.battery_socket_energy_export') | float
    ///     - states('sensor.p1_meter_energy_export') | float
    ///     - states('sensor.battery_socket_energy_import') | float
    /// }}
    ///
    /// [1]: https://developers.home-assistant.io/docs/api/rest/
    /// [2]: https://developers.home-assistant.io/docs/core/entity/sensor/#available-state-classes
    /// ```
    total_energy_usage_url: Url,
}

impl Api {
    #[expect(dead_code)]
    pub fn try_new(access_token: &str, total_energy_usage_url: Url) -> Result<Self> {
        let headers = HeaderMap::from_iter([(
            HeaderName::from_static("Authorization"),
            HeaderValue::from_str(access_token)?,
        )]);
        let client = ClientBuilder::new()
            .default_headers(headers)
            .danger_accept_invalid_certs(true) // FIXME
            .danger_accept_invalid_hostnames(true) // FIXME
            .build()?;
        Ok(Self { client, total_energy_usage_url })
    }
}

#[must_use]
#[serde_as]
#[derive(serde::Deserialize)]
struct State {
    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[serde(rename = "state")]
    value: f64,

    #[serde(rename = "last_reported")]
    last_reported_at: DateTime<Local>,

    attributes: StateAttributes,
}

#[derive(serde::Deserialize)]
struct StateAttributes {
    #[serde(rename = "state_class")]
    class: StateClass,
}

/// [State classes][1].
///
/// [1]: https://developers.home-assistant.io/docs/core/entity/sensor/#available-state-classes
#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Deserialize)]
enum StateClass {
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
