use chrono::{DateTime, DurationRound, Local, TimeDelta};
use reqwest::Client;
use serde::Deserialize;
use serde_with::serde_as;

use crate::{core::point::Point, prelude::*, units::power_density::PowerDensity};

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
        let (live, mut hourly) = {
            let response = self.client.get(&self.url).send().await?.json::<Response>().await?;
            (response.live, response.hourly_forecast)
        };

        // Sometimes, they return a past forecast…
        hourly.retain(|forecast| forecast.timestamp >= since);

        // And, correct for when the current hour forecast disappears:
        let maybe_first = match hourly.first() {
            Some(first) if first.timestamp == since => {
                // No need to correct the forecast:
                None
            }
            _ => match live.first() {
                Some(live) => {
                    warn!("Missing forecast for the current hour, using live weather");
                    Some(Point::try_from(live)?)
                }
                _ => {
                    bail!("missing both forecasted and live weather for the current hour");
                }
            },
        };

        Ok(maybe_first.into_iter().chain(hourly.into_iter().map(Point::from)).collect())
    }
}

#[derive(Deserialize)]
struct Response {
    #[serde(rename = "liveweer")]
    live: Vec<Live>,

    #[serde(rename = "uur_verw")]
    hourly_forecast: Vec<HourlyForecast>,
}

#[serde_as]
#[derive(Copy, Clone, Deserialize)]
struct Live {
    #[serde_as(as = "serde_with::TimestampSeconds<i64>")]
    #[serde(rename = "timestamp")]
    timestamp: DateTime<Local>,

    #[serde(rename = "gr")]
    solar_power_watts_per_m2: f64,
}

impl TryFrom<&Live> for Point<PowerDensity> {
    type Error = Error;

    fn try_from(live: &Live) -> Result<Self> {
        Ok(Self::new(
            live.timestamp.duration_trunc(TimeDelta::hours(1))?,
            PowerDensity::from_watts(live.solar_power_watts_per_m2),
        ))
    }
}

#[serde_as]
#[derive(Copy, Clone, Deserialize)]
struct HourlyForecast {
    #[serde_as(as = "serde_with::TimestampSeconds<i64>")]
    #[serde(rename = "timestamp")]
    timestamp: DateTime<Local>,

    #[serde(rename = "gr")]
    solar_power_watts_per_m2: f64,
}

impl From<HourlyForecast> for Point<PowerDensity> {
    fn from(forecast: HourlyForecast) -> Self {
        Self::new(forecast.timestamp, PowerDensity::from_watts(forecast.solar_power_watts_per_m2))
    }
}
