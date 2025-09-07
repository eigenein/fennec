//! [NextEnergy](https://www.nextenergy.nl/actuele-energieprijzen) client.

use std::str::FromStr;

use chrono::{NaiveDate, NaiveDateTime, TimeDelta, Timelike};
use reqwest::Client;
use serde::{Deserialize, Deserializer, Serialize, de};

use crate::{prelude::*, units::EuroPerKilowattHour};

pub struct NextEnergy(Client);

impl NextEnergy {
    pub fn try_new() -> Result<Self> {
        Ok(Self(Client::builder().build()?))
    }

    /// Fetch the next rates for up to 48 hours and sort them by the respective time slots.
    #[instrument(name = "Fetching energy prices…", fields(since = since.to_string()), skip_all)]
    pub async fn get_upcoming_hourly_rates(&self, since: NaiveDateTime) -> Result<Vec<HourlyRate>> {
        let mut rates: Vec<_> = self
            .get_hourly_rates(since.date())
            .await
            .context("failed to fetch rates for today")?
            .into_iter()
            .filter(|rate| rate.start_at.hour() >= since.hour())
            .collect();
        rates.extend(self.get_hourly_rates(since.date() + TimeDelta::days(1)).await?);
        rates.sort_by_key(|point| point.start_at);
        info!("Fetched", n_rates = rates.len().to_string(), since = since.to_string());
        Ok(rates)
    }

    #[instrument(name = "Fetching energy prices…", fields(date = date.to_string()), skip_all)]
    async fn get_hourly_rates(&self, date: NaiveDate) -> Result<Vec<HourlyRate>> {
        let response: GetDataPointsResponse = self.0.post("https://mijn.nextenergy.nl/Website_CW/screenservices/Website_CW/MainFlow/WB_EnergyPrices/DataActionGetDataPoints")
            .header("X-CSRFToken", "T6C+9iB49TLra4jEsMeSckDMNhQ=")
            .json(&GetDataPointsRequest::new(date))
            .send()
            .await
            .context("failed to call")?
            .error_for_status()
            .context("request failed")?
            .json()
            .await
            .context("failed to deserialize the response")?;

        info!(
            "Fetched",
            len = response.data.points.list.len().to_string(),
            date = date.to_string(),
        );
        response
            .data
            .points
            .list
            .into_iter()
            .map(|point| HourlyRate::try_from_data_point(date, &point))
            .collect()
    }
}

#[derive(Copy, Clone)]
pub struct HourlyRate {
    pub start_at: NaiveDateTime,
    pub value: EuroPerKilowattHour,
}

impl HourlyRate {
    fn try_from_data_point(
        date: NaiveDate,
        data_point: &GetDataPointsResponseDataPoint,
    ) -> Result<Self> {
        Ok(Self {
            start_at: date
                .and_hms_opt(data_point.hour, 0, 0)
                .context("incorrect data point label")?,
            value: data_point.value,
        })
    }
}

#[derive(Deserialize)]
struct GetDataPointsResponse {
    data: GetDataPointsResponseData,
}

#[derive(Deserialize)]
struct GetDataPointsResponseData {
    #[serde(rename = "DataPoints")]
    points: GetDataPointsResponseDataPoints,
}

#[derive(Deserialize)]
struct GetDataPointsResponseDataPoints {
    #[serde(rename = "List")]
    list: Vec<GetDataPointsResponseDataPoint>,
}

#[derive(Deserialize)]
struct GetDataPointsResponseDataPoint {
    #[serde(
        rename = "Label",
        deserialize_with = "GetDataPointsResponseDataPoint::deserialize_label"
    )]
    hour: u32,

    /// Kilowatt-hour rate.
    #[serde(rename = "Value")]
    value: EuroPerKilowattHour,
}

impl GetDataPointsResponseDataPoint {
    fn deserialize_label<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
        let label = String::deserialize(deserializer)?;
        u32::from_str(&label)
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(&label), &"a valid integer"))
    }
}

#[derive(Serialize)]
struct GetDataPointsRequest<'a> {
    #[serde(rename = "viewName")]
    pub view_name: &'a str,

    #[serde(rename = "versionInfo")]
    pub version_info: VersionInfo<'a>,

    #[serde(rename = "screenData")]
    pub screen_data: ScreenData,
}

impl GetDataPointsRequest<'_> {
    pub const fn new(date: NaiveDate) -> Self {
        Self {
            view_name: "MainFlow.MarketPrices",
            version_info: VersionInfo {
                api_version: "4fAioRaV8iwFjjxeuz4+vw",
                module_version: "4m7kd3sh6JgpFidC7o2TPA",
            },
            screen_data: ScreenData {
                variables: Variables { distribution_id: 3, filter_price_date: date },
            },
        }
    }
}

#[derive(Serialize)]
struct VersionInfo<'a> {
    #[serde(rename = "apiVersion")]
    api_version: &'a str,

    #[serde(rename = "moduleVersion")]
    module_version: &'a str,
}

#[derive(Serialize)]
struct ScreenData {
    #[serde(rename = "variables")]
    variables: Variables,
}

#[derive(Serialize)]
struct Variables {
    #[serde(rename = "DistributionId")]
    distribution_id: u8,

    #[serde(rename = "Filter_PriceDate")]
    filter_price_date: NaiveDate,
}

#[cfg(test)]
mod tests {
    use chrono::Local;

    use super::*;

    #[tokio::test]
    #[ignore = "makes the API request"]
    async fn test_get_hourly_rates_ok() -> Result {
        let points = NextEnergy::try_new()?.get_hourly_rates(Local::now().date_naive()).await?;
        assert_eq!(points.len(), 24);
        Ok(())
    }
}
