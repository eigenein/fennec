use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::{ops::Interval, prelude::*, quantity::rate::KilowattHourRate};

pub struct Api {
    client: reqwest::Client,
    resolution: Resolution,
}

impl Api {
    pub fn new(resolution: Resolution) -> Result<Self> {
        let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;
        Ok(Self { client, resolution })
    }

    #[instrument(skip_all)]
    pub async fn get_rates(&self, on: NaiveDate) -> Result<Vec<(Interval, KilowattHourRate)>> {
        info!(?on, "fetching…");
        let Some(data) = self
            .client
            .post("https://www.frankenergie.nl/graphql")
            .json(&Request::new(on, self.resolution))
            .send()
            .await?
            .json::<Response>()
            .await?
            .data
        else {
            return Ok(Vec::new());
        };
        Ok(data
            .market_prices
            .electricity
            .into_iter()
            .map(|item| {
                (Interval::from_std(item.from..item.till), KilowattHourRate::from(item.all_in))
            })
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

    #[tokio::test]
    #[ignore = "makes the API request"]
    async fn test_get_upcoming_rates_ok() -> Result {
        let series = Api::new(Resolution::Quarterly)?.get_rates(Local::now().date_naive()).await?;
        assert!(!series.is_empty());
        assert!(series.len() <= 24 * 4);
        let (time_range, _) = &series[0];
        assert_eq!(time_range.start.hour(), 0);
        assert!(series.iter().is_sorted_by_key(|(time_range, _)| time_range.start));
        Ok(())
    }
}
