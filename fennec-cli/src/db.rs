use std::fmt::Debug;

use bson::doc;
use mongodb::{Client, Database, error::ErrorKind, options::TimeseriesOptions};

use crate::{
    db::{battery_log::BatteryLogs, state::States},
    prelude::*,
};

pub mod battery_log;
pub mod legacy_db;
pub mod legacy_key;
pub mod scalars;
pub mod selectable;
pub mod state;
mod timestamp;

#[must_use]
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
        BatteryLogs::initialize_on(&this).await?;
        Ok(this)
    }

    #[instrument(skip_all, fields(name = name))]
    pub(self) async fn create_timeseries(&self, name: &str, options: TimeseriesOptions) -> Result {
        self.0.create_collection(name).timeseries(options).await.or_else(|error| {
            match error.kind.as_ref() {
                ErrorKind::Command(error) if error.code == 48 => {
                    warn!("collection already exists");
                    Ok(())
                }
                _ => Err(error),
            }
        })?;
        self.0
            .run_command(doc! {
                "collMod": name,
                "expireAfterSeconds": 365 * 24 * 60 * 60, // FIXME: make configurable.
            })
            .await?;
        Ok(())
    }

    /// Get the application state collection.
    pub fn states(&self) -> States {
        States(self.0.collection(States::COLLECTION_NAME))
    }

    pub fn battery_logs(&self) -> BatteryLogs {
        BatteryLogs(self.0.collection(BatteryLogs::COLLECTION_NAME))
    }
}
