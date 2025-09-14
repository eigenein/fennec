use chrono::{DateTime, DurationRound, Local, TimeDelta};
use reqwest::Client;
use serde::Deserialize;
use serde_with::serde_as;

use crate::{prelude::*, strategy::Point, units::PowerDensity};

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

    #[instrument(skip_all, name = "Fetching the local weather…", fields(since = ?since))]
    pub async fn get(&self, since: DateTime<Local>) -> Result<Vec<Point<PowerDensity>>> {
        let since = since.duration_trunc(TimeDelta::hours(1))?;
        let forecast: Vec<_> = self
            .client
            .get(&self.url)
            .send()
            .await?
            .json::<Response>()
            .await?
            .hourly_forecast
            .into_iter()
            .collect();
        ensure!(forecast.is_sorted_by_key(|entry| entry.start_time), "the forecast is not sorted");
        ensure!(forecast.first().context("missing forecast")?.start_time == since); // FIXME
        Ok(forecast
            .into_iter()
            .map(|entry| Point {
                time: entry.start_time,
                value: PowerDensity::from(entry.solar_power_watts_per_m2 / 1000.0),
            })
            .collect())
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
