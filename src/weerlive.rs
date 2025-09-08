use chrono::{NaiveDateTime, Timelike};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, de::Unexpected};

use crate::{prelude::*, units::power::KilowattsPerMeterSquared};

pub struct Weerlive {
    client: Client,
    url: String,
}

pub enum Location {
    #[allow(dead_code)]
    Name(&'static str),

    Coordinates {
        latitude: Decimal,
        longitude: Decimal,
    },
}

impl Location {
    pub const fn coordinates(latitude: Decimal, longitude: Decimal) -> Self {
        Self::Coordinates { latitude, longitude }
    }
}

impl Weerlive {
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
    pub async fn get(&self, now: NaiveDateTime) -> Result<Vec<KilowattsPerMeterSquared>> {
        let mut hourly_forecast: Vec<_> = self
            .client
            .get(&self.url)
            .send()
            .await?
            .json::<Forecast>()
            .await?
            .hourly_forecast
            .into_iter()
            .filter(|forecast| {
                let start_time = forecast.start_time;
                // Keep the future forecasts:
                start_time > now
                // And the current hour forecast:
                || (start_time.date() == now.date() && start_time.hour() == now.hour())
            })
            .collect();
        hourly_forecast.sort_by_key(|entry| entry.start_time);
        match hourly_forecast.first() {
            Some(next_hour_forecast) => {
                // At some point, Weerlive stops returning any forecast for the current hour:
                let next_hour = next_hour_forecast.start_time.hour();
                if next_hour != now.hour() {
                    // Use the next hour as a predictor for the current hour:
                    ensure!(next_hour == (now.hour() + 1) % 24);
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
                    solar_power_watts_per_m2 = forecast.solar_power_watts_per_m2.to_string(),
                );
            })
            .map(|entry| KilowattsPerMeterSquared(entry.solar_power_watts_per_m2 / 1000.0))
            .collect())
    }
}

#[derive(Deserialize)]
struct Forecast {
    #[serde(rename = "uur_verw")]
    hourly_forecast: Vec<HourlyForecast>,
}

#[derive(Copy, Clone, Deserialize)]
struct HourlyForecast {
    #[serde(rename = "uur", deserialize_with = "deserialize_start_time")]
    start_time: NaiveDateTime,

    #[serde(rename = "gr")]
    solar_power_watts_per_m2: f64,
}

fn deserialize_start_time<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let string = String::deserialize(deserializer)?;
    NaiveDateTime::parse_from_str(&string, "%d-%m-%Y %H:%M")
        .map_err(|_| serde::de::Error::invalid_value(Unexpected::Str(&string), &"valid date/time"))
}

#[cfg(test)]
mod tests {
    use chrono::{Local, Timelike};

    use super::*;

    #[tokio::test]
    #[ignore = "online test"]
    async fn test_get_ok() -> Result {
        let now = Local::now().naive_local();
        Weerlive::new("demo", &Location::Name("Amsterdam")).get(now).await?;
        // TODO: add assertions.
        Ok(())
    }
}
