use mongodb::Client;

use crate::prelude::*;

pub mod battery_log;
pub mod legacy_db;
pub mod legacy_key;
pub mod scalars;
pub mod selectable;

pub struct Db(Client);

impl Db {
    pub async fn with_uri_str(uri: &str) -> Result<Self> {
        let inner = Client::with_uri_str(uri).await?;
        Ok(Self(inner))
    }
}
