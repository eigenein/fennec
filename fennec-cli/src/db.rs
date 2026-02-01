use std::fmt::Debug;

use mongodb::{Client, Database};

use crate::{db::state::States, prelude::*};

pub mod battery_log;
pub mod legacy_db;
pub mod legacy_key;
pub mod scalars;
pub mod selectable;
pub mod state;

#[must_use]
pub struct Db(Database);

impl Db {
    /// Connect to the database with the specified URI.
    ///
    /// The URO *must* specify the database name.
    #[instrument]
    pub async fn with_uri(uri: impl AsRef<str> + Debug) -> Result<Self> {
        Client::with_uri_str(uri)
            .await?
            .default_database()
            .context("MongoDB URI does not define the default database")
            .map(Self)
    }

    /// Get the application state collection.
    pub fn states(&self) -> States {
        States(self.0.collection("states"))
    }
}
