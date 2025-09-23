//! [NextEnergy](https://www.nextenergy.nl/actuele-energieprijzen) client.

use std::str::FromStr;

use chrono::{DateTime, DurationRound, Local, NaiveDate, TimeDelta};
use reqwest::Client;
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_with::serde_as;

use crate::{core::series::Series, prelude::*, units::rate::KilowattHourRate};

pub struct Api(Client);

impl Api {
    pub fn try_new() -> Result<Self> {
        Ok(Self(Client::builder().build()?))
    }

    pub async fn get_hourly_rates_48h(
        &self,
        since: DateTime<Local>,
    ) -> Result<Series<KilowattHourRate>> {
        // Round down to the closest hour:
        let since = since.duration_trunc(TimeDelta::hours(1))?;
        let this_day_rates = self.get_hourly_rates(since.date_naive()).await?;

        let next_day = (since + TimeDelta::days(1)).duration_trunc(TimeDelta::days(1))?;
        let next_day_rates = self.get_hourly_rates(next_day.date_naive()).await?;

        Ok(this_day_rates
            .into_iter()
            .filter(|(timestamp, _)| *timestamp >= since)
            .chain(next_day_rates)
            .collect())
    }

    #[instrument(name = "Fetching energy prices…", fields(on = ?on), skip_all)]
    pub async fn get_hourly_rates(&self, on: NaiveDate) -> Result<Series<KilowattHourRate>> {
        Ok(self.0.post("https://mijn.nextenergy.nl/Website_CW/screenservices/Website_CW/MainFlow/WB_EnergyPrices/DataActionGetDataPoints")
            .header("X-CSRFToken", "T6C+9iB49TLra4jEsMeSckDMNhQ=")
            .json(&GetDataPointsRequest::new(on))
            .send()
            .await
            .context("failed to call")?
            .error_for_status()
            .context("request failed")?
            .json::<GetDataPointsResponse>()
            .await
            .context("failed to deserialize the response")?
            .data
            .points
            .list
            .into_iter()
            .map(|point| (on.and_hms_opt(point.hour, 0, 0).context("invalid timestamp").unwrap().and_local_timezone(Local).unwrap(), point.value))
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

#[serde_as]
#[derive(Deserialize)]
struct GetDataPointsResponseDataPoint {
    #[serde(
        rename = "Label",
        deserialize_with = "GetDataPointsResponseDataPoint::deserialize_label"
    )]
    hour: u32,

    /// Kilowatt-hour rate.
    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[serde(rename = "Value")]
    value: KilowattHourRate,
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
    use chrono::{Local, Timelike};

    use super::*;

    #[tokio::test]
    #[ignore = "makes the API request"]
    async fn test_get_hourly_rates_48h_ok() -> Result {
        let now = Local::now();
        let series = Api::try_new()?.get_hourly_rates_48h(now).await?;
        assert!(series.len() >= 24);
        assert!(series.len() <= 48);
        let (timestamp, _) = series.iter().next().unwrap();
        assert_eq!(timestamp.hour(), now.hour());
        Ok(())
    }
}
