use std::ops::{Div, Mul};

use chrono::{DateTime, Local, TimeDelta};
use reqwest::{
    Client,
    ClientBuilder,
    Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, de::DeserializeOwned};
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
    pub async fn get_history<A: DeserializeOwned>(
        &self,
        entity_id: &str,
        from: DateTime<Local>,
        until: DateTime<Local>,
    ) -> Result<EntityHistory<A>> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|()| anyhow!("invalid base URL"))?
            .push("history")
            .push("period")
            .push(&from.to_rfc3339());
        url.query_pairs_mut()
            .append_pair("filter_entity_id", entity_id)
            .append_pair("end_time", &until.to_rfc3339());
        let entities_history: EntitiesHistory<A> =
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
struct EntitiesHistory<A>(
    #[serde(bound(deserialize = "A: DeserializeOwned"))] pub Vec<EntityHistory<A>>,
);

#[must_use]
#[serde_as]
#[derive(Deserialize, derive_more::Index, derive_more::IntoIterator)]
pub struct EntityHistory<A>(
    #[serde_as(as = "serde_with::VecSkipError<_>")]
    #[serde(bound(deserialize = "A: DeserializeOwned"))]
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

#[must_use]
#[derive(Copy, Clone, derive_more::Add, derive_more::Sub, serde::Deserialize)]
pub struct BatteryStateAttributes<T> {
    #[serde(rename = "custom_battery_residual_energy")]
    pub residual_energy: T,

    #[serde(rename = "custom_battery_energy_import")]
    pub total_import: T,

    #[serde(rename = "custom_battery_energy_export")]
    pub total_export: T,
}

impl Div<TimeDelta> for BatteryStateAttributes<KilowattHours> {
    type Output = BatteryStateAttributes<Kilowatts>;

    fn div(self, rhs: TimeDelta) -> Self::Output {
        BatteryStateAttributes {
            residual_energy: self.residual_energy / rhs,
            total_import: self.total_import / rhs,
            total_export: self.total_export / rhs,
        }
    }
}

impl Mul<TimeDelta> for BatteryStateAttributes<Kilowatts> {
    type Output = BatteryStateAttributes<KilowattHours>;

    fn mul(self, rhs: TimeDelta) -> Self::Output {
        BatteryStateAttributes {
            residual_energy: self.residual_energy * rhs,
            total_import: self.total_import * rhs,
            total_export: self.total_export * rhs,
        }
    }
}

#[must_use]
#[derive(Copy, Clone, derive_more::Add, derive_more::Sub)]
pub struct EnergyState<T> {
    /// Net household energy usage excluding the energy systems.
    pub total_energy_usage: T,

    pub battery: BatteryStateAttributes<T>,
}

impl<V: From<f64>> From<State<BatteryStateAttributes<V>>> for (DateTime<Local>, EnergyState<V>) {
    /// Unpack the state for collection into a series.
    fn from(state: State<BatteryStateAttributes<V>>) -> Self {
        (
            state.last_changed_at,
            EnergyState { total_energy_usage: state.value.into(), battery: state.attributes },
        )
    }
}

impl Div<TimeDelta> for EnergyState<KilowattHours> {
    type Output = EnergyState<Kilowatts>;

    fn div(self, rhs: TimeDelta) -> Self::Output {
        EnergyState {
            total_energy_usage: self.total_energy_usage / rhs,
            battery: self.battery / rhs,
        }
    }
}

impl Mul<TimeDelta> for EnergyState<Kilowatts> {
    type Output = EnergyState<KilowattHours>;

    fn mul(self, rhs: TimeDelta) -> Self::Output {
        EnergyState {
            total_energy_usage: self.total_energy_usage * rhs,
            battery: self.battery * rhs,
        }
    }
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
