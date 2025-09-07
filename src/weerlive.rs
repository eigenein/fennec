use chrono::{Local, NaiveDateTime, Timelike};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, de::Unexpected};

use crate::{prelude::*, units::power::KilowattsPerMeterSquared};

pub struct Weerlive {
    client: Client,
    url: String,
}

pub enum Location {
    Name(&'static str),
    Coordinates { latitude: Decimal, longitude: Decimal },
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

    #[instrument(skip_all, name = "Fetching the local weather…")]
    pub async fn get(&self, start_hour: u32) -> Result<Vec<KilowattsPerMeterSquared>> {
        let mut hourly_forecast =
            self.client.get(&self.url).send().await?.json::<Forecast>().await?.hourly_forecast;
        hourly_forecast.sort_by_key(|entry| entry.start_time);
        match hourly_forecast.first() {
            Some(forecast) => {
                let actual_start_hour = forecast.start_time.hour();
                if actual_start_hour != start_hour {
                    // At some point, Weerlive stops returning any forecast for the current hour:
                    ensure!(actual_start_hour == start_hour + 1);
                    // Use the next hour as a predictor for the current hour:
                    hourly_forecast.insert(0, *forecast);
                }
            }
            None => {
                bail!("there is no forecast");
            }
        }
        Ok(hourly_forecast
            .into_iter()
            .inspect(|forecast| {
                info!(
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
    use chrono::Timelike;

    use super::*;

    #[tokio::test]
    #[ignore = "online test"]
    async fn test_get_ok() -> Result {
        let start_hour = Local::now().naive_local().hour();
        Weerlive::new("demo", &Location::Name("Amsterdam")).get(start_hour).await?;
        Ok(())
    }
}
