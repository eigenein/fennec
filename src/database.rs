use chrono::Local;
use mongodb::Client;
use reqwest::Url;

use crate::{prelude::*, quantity::energy::KilowattHours};

pub struct Database(mongodb::Database);

impl Database {
    #[instrument(skip_all, name = "Connecting to the database…")]
    pub async fn try_new(url: &Url) -> Result<Self> {
        Ok(Self(
            Client::with_uri_str(url).await?.default_database().context("missing database name")?,
        ))
    }

    #[instrument(skip_all, name = "Logging the total energy usage reading…", fields(value = %value))]
    pub async fn log_total_energy_usage(
        &self,
        timestamp: chrono::DateTime<Local>,
        value: KilowattHours,
    ) -> Result {
        self.0
            .collection::<TotalEnergyUsageReading>("totalEnergyUsage")
            .insert_one(&TotalEnergyUsageReading {
                timestamp: mongodb::bson::DateTime::from_millis(timestamp.timestamp_millis()),
                value,
            })
            .await?;
        Ok(())
    }
}

#[derive(serde::Serialize)]
struct TotalEnergyUsageReading {
    #[serde(rename = "_id")]
    timestamp: mongodb::bson::DateTime,

    value: KilowattHours,
}
