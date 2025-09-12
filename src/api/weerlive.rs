use chrono::{DateTime, Local, Timelike};
use reqwest::Client;
use serde::Deserialize;
use serde_with::serde_as;

use crate::{prelude::*, units::PowerDensity};

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

    #[instrument(skip_all, name = "Fetching the local weather…", fields(starting_hour = starting_hour))]
    pub async fn get(&self, starting_hour: u32) -> Result<Vec<PowerDensity>> {
        let mut hourly_forecast: Vec<_> =
            self.client.get(&self.url).send().await?.json::<Forecast>().await?.hourly_forecast;
        hourly_forecast.sort_by_key(|entry| entry.start_time);
        match hourly_forecast.first() {
            Some(next_hour_forecast) => {
                // At some point, Weerlive stops returning any forecast for the current hour:
                let next_hour = next_hour_forecast.start_time.hour();
                if next_hour != starting_hour {
                    // Use the next hour as a predictor for the current hour:
                    ensure!(next_hour == (starting_hour + 1) % 24);
                    hourly_forecast.insert(0, *next_hour_forecast);
                }
            }
            None => {
                bail!("there is no forecast");
            }
        }
        Ok(hourly_forecast
            .into_iter()
            .inspect(|forecast| {
                debug!(
                    "Forecast",
                    hour = forecast.start_time.hour().to_string(),
                    solar_power_watts_per_m2 = forecast.solar_power_watts_per_m2,
                );
            })
            .map(|entry| PowerDensity::from(entry.solar_power_watts_per_m2 / 1000.0))
            .collect())
    }
}

#[derive(Deserialize)]
struct Forecast {
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
    use chrono::Local;

    use super::*;

    #[tokio::test]
    #[ignore = "online test"]
    async fn test_get_ok() -> Result {
        let now = Local::now();
        Api::new("demo", &Location::Name("Amsterdam")).get(now.hour()).await?;
        // TODO: add assertions.
        Ok(())
    }
}
