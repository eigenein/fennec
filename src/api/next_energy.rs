//! [NextEnergy](https://www.nextenergy.nl/actuele-energieprijzen) client.

use std::{str::FromStr, time::Duration};

use chrono::{Local, MappedLocalTime, NaiveDate, TimeDelta};
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_with::serde_as;
use ureq::Agent;

use crate::{
    core::series::Point,
    prelude::*,
    quantity::{interval::Interval, rate::KilowattHourRate},
};

pub struct Api(Agent);

impl Api {
    pub fn new() -> Self {
        let client =
            Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).build().into();
        Self(client)
    }

    /// Get all hourly rates on the specified day.
    #[instrument(fields(on = ?on), skip_all)]
    pub fn get_rates(&self, on: NaiveDate) -> Result<Vec<Point<Interval, KilowattHourRate>>> {
        info!("Fetching…");
        let data_points = self.0.post("https://mijn.nextenergy.nl/Website_CW/screenservices/Website_CW/Blocks/WB_EnergyPrices/DataActionGetDataPoints")
            .header("X-CSRFToken", "T6C+9iB49TLra4jEsMeSckDMNhQ=")
            .send_json(GetDataPointsRequest::new(on))
            .context("request failed")?
            .body_mut()
            .read_json::<GetDataPointsResponse>()
            .context("failed to deserialize the response")?
            .data
            .points
            .list;
        info!(n_data_points = data_points.len(), "Fetched");
        let series = data_points.into_iter().enumerate().filter_map(move |(index, point)| {
            let hour = u32::try_from(index).unwrap();
            assert_eq!(
                (point.label + 1) % 24,
                hour,
                "NextEnergy messed up: index={index} label={}",
                point.label
            );

            match on.and_hms_nano_opt(hour, 0, 0, 0).unwrap().and_local_timezone(Local) {
                MappedLocalTime::Single(start_time) | MappedLocalTime::Ambiguous(start_time, _) => {
                    let end_time = start_time + TimeDelta::hours(1);
                    let point = (Interval::new(start_time, end_time), point.value);
                    Some(point)
                }

                MappedLocalTime::None => {
                    warn!(point.label, index, "Skipped");
                    None
                }
            }
        });
        Ok(series.collect())
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

    use super::*;

    #[test]
    #[ignore = "makes the API request"]
    fn test_get_upcoming_rates_ok() -> Result {
        let now = Local::now();
        let series = Api::new().get_rates(now.date_naive())?;
        assert!(series.len() >= 1);
        assert!(series.len() <= 24);
        let (time_range, _) = &series[0];
        assert_eq!(time_range.start.hour(), now.hour());
        assert!(series.iter().is_sorted_by_key(|(time_range, _)| time_range.start));
        Ok(())
    }
}
