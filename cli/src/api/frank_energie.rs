use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate};
use quantities::{interval::Interval, rate::KilowattHourRate};
use serde::{Deserialize, Serialize};
use ureq::Agent;

use crate::prelude::*;

pub struct Api {
    client: Agent,
    resolution: Resolution,
}

impl Api {
    pub fn new(resolution: Resolution) -> Self {
        let client =
            Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).build().into();
        Self { client, resolution }
    }

    #[instrument(fields(on = ?on), skip_all)]
    pub fn get_rates(&self, on: NaiveDate) -> Result<Vec<(Interval, KilowattHourRate)>> {
        info!("Fetching…");
        let Some(data) = self
            .client
            .post("https://www.frankenergie.nl/graphql")
            .send_json(Request::new(on, self.resolution))?
            .body_mut()
            .read_json::<Response>()?
            .data
        else {
            return Ok(Vec::new());
        };
        Ok(data
            .market_prices
            .electricity
            .into_iter()
            .map(|item| (Interval::new(item.from, item.till), KilowattHourRate::from(item.all_in)))
            .collect())
    }
}

#[derive(Serialize)]
struct Request {
    #[serde(rename = "MarketPrices")]
    operation_name: &'static str,

    query: &'static str,

    variables: Variables,
}

impl Request {
    const fn new(date: NaiveDate, resolution: Resolution) -> Self {
        Self {
            operation_name: "MarketPrices",
            query: "query MarketPrices($date: String!, $resolution: PriceResolution!) { marketPrices(date: $date, resolution: $resolution) { electricityPrices { from till allInPrice } } }",
            variables: Variables { date, resolution },
        }
    }
}

#[derive(Serialize)]
struct Variables {
    date: NaiveDate,
    resolution: Resolution,
}

#[derive(Copy, Clone, Serialize)]
pub enum Resolution {
    #[serde(rename = "PT15M")]
    Quarterly,

    #[serde(rename = "PT60M")]
    Hourly,
}

#[derive(Deserialize)]
struct Response {
    data: Option<Data>,
}

#[derive(Deserialize)]
struct Data {
    #[serde(rename = "marketPrices")]
    market_prices: MarketPrices,
}

#[derive(Deserialize)]
struct MarketPrices {
    #[serde(rename = "electricityPrices")]
    electricity: Vec<ElectricityPrice>,
}

#[derive(Deserialize)]
struct ElectricityPrice {
    from: DateTime<Local>,
    till: DateTime<Local>,

    #[serde(rename = "allInPrice")]
    all_in: f64,
}

#[cfg(test)]
mod tests {
    use chrono::Timelike;

    use super::*;

    #[test]
    #[ignore = "makes the API request"]
    fn test_get_upcoming_rates_ok() -> Result {
        let series = Api::new(Resolution::Quarterly).get_rates(Local::now().date_naive())?;
        assert!(series.len() >= 1);
        assert!(series.len() <= 24 * 4);
        let (time_range, _) = &series[0];
        assert_eq!(time_range.start.hour(), 0);
        assert!(series.iter().is_sorted_by_key(|(time_range, _)| time_range.start));
        Ok(())
    }
}
