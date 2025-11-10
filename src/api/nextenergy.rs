//! [NextEnergy](https://www.nextenergy.nl/actuele-energieprijzen) client.

use std::{ops::Range, str::FromStr, time::Duration};

use chrono::{DateTime, DurationRound, Local, MappedLocalTime, NaiveDate, TimeDelta};
use reqwest::Client;
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_with::serde_as;

use crate::{core::series::Point, prelude::*, quantity::rate::KilowattHourRate};

pub struct Api(Client);

impl Api {
    pub fn try_new() -> Result<Self> {
        Ok(Self(Client::builder().timeout(Duration::from_secs(10)).build()?))
    }

    pub async fn get_hourly_rates_48h(
        &self,
        since: DateTime<Local>,
    ) -> Result<impl Iterator<Item = Point<Range<DateTime<Local>>, KilowattHourRate>>> {
        // Round down to the closest hour:
        let this_day_rates = self.get_hourly_rates(since.date_naive()).await?;

        let next_day_rates = {
            let next_day = (since + TimeDelta::days(1)).duration_trunc(TimeDelta::days(1))?;
            self.get_hourly_rates(next_day.date_naive()).await?
        };

        Ok(this_day_rates
            .into_iter()
            .filter(move |(time_range, _)| time_range.end >= since)
            .chain(next_day_rates))
    }

    #[instrument(name = "Fetching energy prices…", fields(on = ?on), skip_all)]
    pub async fn get_hourly_rates(
        &self,
        on: NaiveDate,
    ) -> Result<impl Iterator<Item = Point<Range<DateTime<Local>>, KilowattHourRate>>> {
        Ok(self.0.post("https://mijn.nextenergy.nl/Website_CW/screenservices/Website_CW/Blocks/WB_EnergyPrices_NEW/DataActionGetDataPoints")
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
            .enumerate()
            .filter_map(move |(index, point)| {
                let hour = u32::try_from(index).unwrap();
                assert_eq!((point.label + 1) % 24, hour, "NextEnergy messed up: index={index} label={}", point.label);

                match on.and_hms_opt(hour, 0, 0).unwrap().and_local_timezone(Local) {
                    MappedLocalTime::Single(start_time) | MappedLocalTime::Ambiguous(start_time, _) => {
                        let end_time = start_time + TimeDelta::hours(1);
                        let point = (start_time..end_time, point.value);
                        Some(point)
                    },

                    MappedLocalTime::None => None,
                }
            })
        )
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
    label: u32,

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
struct GetDataPointsRequest {
    #[serde(rename = "viewName")]
    pub view_name: &'static str,

    #[serde(rename = "versionInfo")]
    pub version_info: VersionInfo,

    #[serde(rename = "screenData")]
    pub screen_data: ScreenData,
}

impl GetDataPointsRequest {
    pub const fn new(date: NaiveDate) -> Self {
        Self {
            view_name: "MainFlow.MarketPrices",
            version_info: VersionInfo {
                api_version: "4fAioRaV8iwFjjxeuz4+vw",
                module_version: "yM5fgj6F4qiLDuJ6CWsCSg",
            },
            screen_data: ScreenData {
                variables: Variables {
                    distribution_id: 3,
                    filter: Filter { costs_level_id: "Market+", price_including_vat: true, date },
                },
            },
        }
    }
}

#[derive(Serialize)]
struct VersionInfo {
    #[serde(rename = "apiVersion")]
    api_version: &'static str,

    #[serde(rename = "moduleVersion")]
    module_version: &'static str,
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

    #[serde(rename = "Filter")]
    filter: Filter,
}

#[derive(Serialize)]
struct Filter {
    #[serde(rename = "PriceIncludingVAT")]
    price_including_vat: bool,

    #[serde(rename = "PriceDate")]
    date: NaiveDate,

    #[serde(rename = "CostsLevelId")]
    costs_level_id: &'static str,
}

#[cfg(test)]
mod tests {
    use chrono::{Local, Timelike};
    use itertools::Itertools;

    use super::*;

    #[tokio::test]
    #[ignore = "makes the API request"]
    async fn test_get_hourly_rates_48h_ok() -> Result {
        let now = Local::now();
        let series = Api::try_new()?.get_hourly_rates_48h(now).await?.collect_vec();
        assert!(series.len() >= 1);
        assert!(series.len() <= 48);
        let (time_range, _) = &series[0];
        assert_eq!(time_range.start.hour(), now.hour());
        Ok(())
    }
}
