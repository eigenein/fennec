use std::{any::type_name, fmt::Debug};

use bson::doc;
use futures_core::TryStream;
use futures_util::TryStreamExt;
use mongodb::{Client, Database};

use crate::{
    core::interval::Interval,
    db::{battery::BatteryLog, consumption::ConsumptionLog, log::Log},
    prelude::*,
};

pub mod battery;
pub mod consumption;
pub mod log;
pub mod state;

#[must_use]
#[derive(Clone)]
pub struct Db(Database);

impl Db {
    /// Connect to the database with the specified URI.
    ///
    /// The URO *must* specify the database name.
    #[instrument(skip_all)]
    pub async fn with_uri(uri: impl AsRef<str> + Debug) -> Result<Self> {
        let inner = Client::with_uri_str(uri)
            .await?
            .default_database()
            .context("MongoDB URI does not define the default database")?;
        let this = Self(inner);
        BatteryLog::initialize_time_series(&this).await?;
        ConsumptionLog::initialize_time_series(&this).await?;
        Ok(this)
    }

    #[instrument(skip_all)]
    pub async fn find_logs<L: Log>(
        &self,
        interval: Interval,
    ) -> Result<impl TryStream<Ok = L, Error = Error>> {
        info!(type = type_name::<Self>(), ?interval, "querying logs…");
        Ok(self
            .0
            .collection::<L>(L::COLLECTION_NAME)
            .find(doc! { "timestamp": { "$gte": interval.start, "$lt": interval.end } })
            .sort(doc! { "timestamp": -1 })
            .await
            .context("failed to query the battery logs")?
            .map_err(Error::from))
    }
}
