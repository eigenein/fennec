use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::{energy::Flow, ops::Interval, prelude::*, quantity::price::KilowattHourPrice};

pub struct Api {
    client: reqwest::Client,
    resolution: Resolution,
}

impl Api {
    /// See <https://www.frankenergie.nl/nl/kennisbank/zonnepanelen/terugleververgoeding#terugleververgoeding-bij-frank>.
    const PURCHASE_FEE: KilowattHourPrice = KilowattHourPrice(0.0182);

    const VAT: f64 = 1.21;

    pub fn new(resolution: Resolution) -> Result<Self> {
        let client = reqwest::Client::builder().timeout(Duration::from_secs(15)).build()?;
        Ok(Self { client, resolution })
    }

    #[instrument(skip_all, fields(on = ?on))]
    pub async fn get_prices(
        &self,
        on: NaiveDate,
    ) -> Result<Vec<(Interval, Flow<KilowattHourPrice>)>> {
        debug!(?on, "fetching…");
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
                (
                    Interval::from_std(item.from..item.till),
                    Flow {
                        import: item.all_in,
                        // FIXME: from 2027, this becomes just `item.market + Self::PURCHASE_FEE`:
                        export: (item.market + Self::PURCHASE_FEE) * Self::VAT,
                    },
                )
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
            query: "query MarketPrices($date: String!, $resolution: PriceResolution!) { marketPrices(date: $date, resolution: $resolution) { electricityPrices { from till marketPrice allInPrice } } }",
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

#[must_use]
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

    #[serde(rename = "marketPrice")]
    market: KilowattHourPrice,

    #[serde(rename = "allInPrice")]
    all_in: KilowattHourPrice,
}

#[cfg(test)]
mod tests {
    use chrono::Timelike;

    use super::*;

    #[tokio::test]
    #[ignore = "makes the API request"]
    async fn get_prices_ok() -> Result {
        let series = Api::new(Resolution::Quarterly)?.get_prices(Local::now().date_naive()).await?;
        assert!(!series.is_empty());
        assert!(series.len() <= 24 * 4);
        let (time_range, _) = &series[0];
        assert_eq!(time_range.start.hour(), 0);
        assert!(series.iter().is_sorted_by_key(|(time_range, _)| time_range.start));
        Ok(())
    }

    #[test]
    #[expect(clippy::too_many_lines)]
    fn parse_ok() -> Result {
        // language=json
        const RESPONSE: &str = r#"
            {
                "data": {
                    "marketPrices": {
                        "averageElectricityPrices": {
                            "averageMarketPrice": 0.08641,
                            "averageMarketPricePlus": 0.1227,
                            "averageAllInPrice": 0.23355,
                            "perUnit": "KWH",
                            "isWeighted": false
                        },
                        "electricityPrices": [
                            {
                                "from": "2026-04-07T22:00:00.000Z",
                                "till": "2026-04-07T23:00:00.000Z",
                                "marketPrice": 0.10254,
                                "marketPricePlus": 0.14222,
                                "allInPrice": 0.25307,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-07T23:00:00.000Z",
                                "till": "2026-04-08T00:00:00.000Z",
                                "marketPrice": 0.09782,
                                "marketPricePlus": 0.13651,
                                "allInPrice": 0.24736,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T00:00:00.000Z",
                                "till": "2026-04-08T01:00:00.000Z",
                                "marketPrice": 0.0966,
                                "marketPricePlus": 0.13504,
                                "allInPrice": 0.24588,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T01:00:00.000Z",
                                "till": "2026-04-08T02:00:00.000Z",
                                "marketPrice": 0.09848,
                                "marketPricePlus": 0.13731,
                                "allInPrice": 0.24816,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T02:00:00.000Z",
                                "till": "2026-04-08T03:00:00.000Z",
                                "marketPrice": 0.10225,
                                "marketPricePlus": 0.14187,
                                "allInPrice": 0.25272,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T03:00:00.000Z",
                                "till": "2026-04-08T04:00:00.000Z",
                                "marketPrice": 0.11214,
                                "marketPricePlus": 0.15384,
                                "allInPrice": 0.26469,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T04:00:00.000Z",
                                "till": "2026-04-08T05:00:00.000Z",
                                "marketPrice": 0.14378,
                                "marketPricePlus": 0.19212,
                                "allInPrice": 0.30297,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T05:00:00.000Z",
                                "till": "2026-04-08T06:00:00.000Z",
                                "marketPrice": 0.1688,
                                "marketPricePlus": 0.2224,
                                "allInPrice": 0.33325,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T06:00:00.000Z",
                                "till": "2026-04-08T07:00:00.000Z",
                                "marketPrice": 0.1418,
                                "marketPricePlus": 0.18973,
                                "allInPrice": 0.30058,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T07:00:00.000Z",
                                "till": "2026-04-08T08:00:00.000Z",
                                "marketPrice": 0.1094,
                                "marketPricePlus": 0.15052,
                                "allInPrice": 0.26137,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T08:00:00.000Z",
                                "till": "2026-04-08T09:00:00.000Z",
                                "marketPrice": 0.05184,
                                "marketPricePlus": 0.08088,
                                "allInPrice": 0.19172,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T09:00:00.000Z",
                                "till": "2026-04-08T10:00:00.000Z",
                                "marketPrice": -0.00005,
                                "marketPricePlus": 0.01809,
                                "allInPrice": 0.12894,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T10:00:00.000Z",
                                "till": "2026-04-08T11:00:00.000Z",
                                "marketPrice": -0.01857,
                                "marketPricePlus": -0.00432,
                                "allInPrice": 0.10653,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T11:00:00.000Z",
                                "till": "2026-04-08T12:00:00.000Z",
                                "marketPrice": -0.03185,
                                "marketPricePlus": -0.02039,
                                "allInPrice": 0.09046,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T12:00:00.000Z",
                                "till": "2026-04-08T13:00:00.000Z",
                                "marketPrice": -0.02887,
                                "marketPricePlus": -0.01678,
                                "allInPrice": 0.09407,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T13:00:00.000Z",
                                "till": "2026-04-08T14:00:00.000Z",
                                "marketPrice": -0.01266,
                                "marketPricePlus": 0.00283,
                                "allInPrice": 0.11368,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T14:00:00.000Z",
                                "till": "2026-04-08T15:00:00.000Z",
                                "marketPrice": 0.01376,
                                "marketPricePlus": 0.0348,
                                "allInPrice": 0.14565,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T15:00:00.000Z",
                                "till": "2026-04-08T16:00:00.000Z",
                                "marketPrice": 0.09397,
                                "marketPricePlus": 0.13185,
                                "allInPrice": 0.2427,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T16:00:00.000Z",
                                "till": "2026-04-08T17:00:00.000Z",
                                "marketPrice": 0.12956,
                                "marketPricePlus": 0.17492,
                                "allInPrice": 0.28577,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T17:00:00.000Z",
                                "till": "2026-04-08T18:00:00.000Z",
                                "marketPrice": 0.1713,
                                "marketPricePlus": 0.22542,
                                "allInPrice": 0.33627,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T18:00:00.000Z",
                                "till": "2026-04-08T19:00:00.000Z",
                                "marketPrice": 0.16124,
                                "marketPricePlus": 0.21325,
                                "allInPrice": 0.3241,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T19:00:00.000Z",
                                "till": "2026-04-08T20:00:00.000Z",
                                "marketPrice": 0.13729,
                                "marketPricePlus": 0.18427,
                                "allInPrice": 0.29512,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T20:00:00.000Z",
                                "till": "2026-04-08T21:00:00.000Z",
                                "marketPrice": 0.12153,
                                "marketPricePlus": 0.1652,
                                "allInPrice": 0.27605,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T21:00:00.000Z",
                                "till": "2026-04-08T22:00:00.000Z",
                                "marketPrice": 0.11165,
                                "marketPricePlus": 0.15325,
                                "allInPrice": 0.26409,
                                "perUnit": "KWH",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            }
                        ],
                        "gasPrices": [
                            {
                                "from": "2026-04-07T22:00:00.000Z",
                                "till": "2026-04-07T23:00:00.000Z",
                                "marketPrice": 0.48879,
                                "marketPricePlus": 0.6713,
                                "allInPrice": 1.3981,
                                "perUnit": "M3",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-07T23:00:00.000Z",
                                "till": "2026-04-08T00:00:00.000Z",
                                "marketPrice": 0.48879,
                                "marketPricePlus": 0.6713,
                                "allInPrice": 1.3981,
                                "perUnit": "M3",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T00:00:00.000Z",
                                "till": "2026-04-08T01:00:00.000Z",
                                "marketPrice": 0.48879,
                                "marketPricePlus": 0.6713,
                                "allInPrice": 1.3981,
                                "perUnit": "M3",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T01:00:00.000Z",
                                "till": "2026-04-08T02:00:00.000Z",
                                "marketPrice": 0.48879,
                                "marketPricePlus": 0.6713,
                                "allInPrice": 1.3981,
                                "perUnit": "M3",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T02:00:00.000Z",
                                "till": "2026-04-08T03:00:00.000Z",
                                "marketPrice": 0.48879,
                                "marketPricePlus": 0.6713,
                                "allInPrice": 1.3981,
                                "perUnit": "M3",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            },
                            {
                                "from": "2026-04-08T03:00:00.000Z",
                                "till": "2026-04-08T04:00:00.000Z",
                                "marketPrice": 0.48879,
                                "marketPricePlus": 0.6713,
                                "allInPrice": 1.3981,
                                "perUnit": "M3",
                                "marketPricePlusComponents": null,
                                "allInPriceComponents": null
                            }
                        ]
                    }
                }
            }
        "#;
        let _ = serde_json::from_str::<Response>(RESPONSE)?;
        Ok(())
    }
}
