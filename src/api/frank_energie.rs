use std::ops::Range;

use async_trait::async_trait;
use chrono::{DateTime, Local, NaiveDate};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    api::{client, energy_provider::EnergyProvider},
    core::series::Point,
    prelude::*,
    quantity::rate::KilowattHourRate,
};

pub struct Api(Client);

impl Api {
    pub fn try_new() -> Result<Self> {
        Ok(Self(client::try_new()?))
    }
}

#[async_trait]
impl EnergyProvider for Api {
    #[instrument(fields(on = ?on), skip_all)]
    async fn get_rates(
        &self,
        on: NaiveDate,
    ) -> Result<Vec<Point<Range<DateTime<Local>>, KilowattHourRate>>> {
        info!("Fetching…");
        Ok(self
            .0
            .post("https://www.frankenergie.nl/graphql")
            .json(&Request::new(on))
            .send()
            .await?
            .json::<Response>()
            .await?
            .data
            .market_prices
            .electricity
            .into_iter()
            .map(|item| (item.from..item.till, KilowattHourRate::from(item.all_in)))
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
    const fn new(date: NaiveDate) -> Self {
        Self {
            operation_name: "MarketPrices",
            query: "query MarketPrices($date: String!, $resolution: PriceResolution!) { marketPrices(date: $date, resolution: $resolution) { electricityPrices { from till allInPrice } } }",
            variables: Variables::new(date),
        }
    }
}

#[derive(Serialize)]
struct Variables {
    date: NaiveDate,
    resolution: Resolution,
}

impl Variables {
    const fn new(date: NaiveDate) -> Self {
        Self { date, resolution: Resolution::Quarterly }
    }
}

#[derive(Serialize)]
enum Resolution {
    #[serde(rename = "PT15M")]
    Quarterly,

    #[serde(rename = "PT60M")]
    Hourly,
}

#[derive(Deserialize)]
struct Response {
    data: Data,
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
        let now = Local::now();
        let series = Api::try_new()?.get_upcoming_rates(now).await?;
        assert!(series.len() >= 1);
        assert!(series.len() <= 2 * 24 * 4);
        let (time_range, _) = &series[0];
        assert_eq!(time_range.start.hour(), now.hour());
        assert!(series.iter().is_sorted_by_key(|(time_range, _)| time_range.start));
        Ok(())
    }
}
