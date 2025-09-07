//! [NextEnergy](https://www.nextenergy.nl/actuele-energieprijzen) client.

use std::str::FromStr;

use chrono::NaiveDate;
use reqwest::Client;
use serde::{Deserialize, Deserializer, Serialize, de};

use crate::{prelude::*, units::EuroPerKilowattHour};

pub struct NextEnergy(Client);

impl NextEnergy {
    pub fn try_new() -> Result<Self> {
        Ok(Self(Client::builder().build()?))
    }

    #[instrument(name = "Fetching energy prices…", fields(date = date.to_string()), skip_all)]
    pub async fn get_hourly_rates(
        &self,
        date: NaiveDate,
        starting_hour: u32,
    ) -> Result<Vec<EuroPerKilowattHour>> {
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
        Ok(response
            .data
            .points
            .list
            .into_iter()
            .filter(|point| point.hour >= starting_hour)
            .inspect(|point| {
                info!("Rate", hour = point.hour.to_string(), value = point.value.to_string());
            })
            .map(|point| point.value)
            .collect())
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
        let points = NextEnergy::try_new()?.get_hourly_rates(Local::now().date_naive(), 0).await?;
        assert_eq!(points.len(), 24);
        Ok(())
    }
}
