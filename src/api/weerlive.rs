use chrono::{DateTime, Local, Timelike};
use reqwest::Client;
use serde::Deserialize;
use serde_with::serde_as;

use crate::{prelude::*, strategy::Forecast, units::PowerDensity};

pub struct Api {
    client: Client,
    url: String,
}

pub enum Location {
    Coordinates {
        latitude: f64,
        longitude: f64,
    },

    #[allow(dead_code)]
    Name(&'static str),
}

impl Location {
    pub const fn coordinates(latitude: f64, longitude: f64) -> Self {
        Self::Coordinates { latitude, longitude }
    }
}

impl Api {
    pub fn new(api_key: &str, location: &Location) -> Self {
        let url = match location {
            Location::Name(name) => {
                format!("https://weerlive.nl/api/weerlive_api_v2.php?key={api_key}&locatie={name}")
            }
            Location::Coordinates { latitude, longitude } => {
                format!(
                    "https://weerlive.nl/api/weerlive_api_v2.php?key={api_key}&locatie={latitude},{longitude}"
                )
            }
        };
        Self { client: Client::new(), url }
    }

    #[instrument(skip_all, name = "Fetching the local weather…", fields(now = ?now))]
    pub async fn get(&self, now: DateTime<Local>) -> Result<Forecast<PowerDensity>> {
        let forecast: Vec<_> = self
            .client
            .get(&self.url)
            .send()
            .await?
            .json::<Response>()
            .await?
            .hourly_forecast
            .into_iter()
            .filter(|entry| Self::is_actual(entry.start_time, now))
            .collect();
        ensure!(forecast.is_sorted_by_key(|entry| entry.start_time), "the forecast is not sorted");
        ensure!(
            forecast.first().context("missing forecast")?.start_time.hour() == now.hour(),
            "the forecast does not start with the current hour",
        );
        let metrics = forecast
            .into_iter()
            .map(|entry| PowerDensity::from(entry.solar_power_watts_per_m2 / 1000.0))
            .collect();
        Ok(Forecast { start_hour: now.hour() as usize, metrics })
    }

    /// Check whether the time slot is still actual.
    ///
    /// This is needed because Weerlive sometimes returns the past hours.
    fn is_actual(slot_time: DateTime<Local>, now: DateTime<Local>) -> bool {
        (slot_time.date_naive(), slot_time.hour()) >= (now.date_naive(), now.hour())
    }
}

#[derive(Deserialize)]
struct Response {
    #[serde(rename = "uur_verw")]
    hourly_forecast: Vec<HourlyForecast>,
}

#[serde_as]
#[derive(Copy, Clone, Deserialize)]
struct HourlyForecast {
    #[serde_as(as = "serde_with::TimestampSeconds<i64>")]
    #[serde(rename = "timestamp")]
    start_time: DateTime<Local>,

    #[serde(rename = "gr")]
    solar_power_watts_per_m2: f64,
}

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone};

    use super::*;

    #[test]
    fn test_is_actual() {
        assert!(!Api::is_actual(
            Local.with_ymd_and_hms(2025, 9, 13, 18, 59, 59).unwrap(),
            Local.with_ymd_and_hms(2025, 9, 13, 19, 19, 0).unwrap(),
        ));
        assert!(Api::is_actual(
            Local.with_ymd_and_hms(2025, 9, 13, 19, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2025, 9, 13, 19, 19, 0).unwrap(),
        ));
        assert!(Api::is_actual(
            Local.with_ymd_and_hms(2025, 9, 14, 19, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2025, 9, 13, 19, 19, 0).unwrap(),
        ));
    }

    #[tokio::test]
    #[ignore = "online test"]
    async fn test_get_ok() -> Result {
        let now = Local::now();
        let forecast = Api::new("demo", &Location::Name("Amsterdam")).get(now).await?;
        assert_eq!(forecast.start_hour, now.hour() as usize);
        // TODO: add assertions.
        Ok(())
    }
}
