use clap::Parser;
use reqwest::Url;

use crate::{db::Db, prelude::*};

#[derive(Parser)]
pub struct DbArgs {
    #[clap(long = "mongodb-uri", env = "MONGODB_URI")]
    uri: Url,
}

impl DbArgs {
    pub async fn connect(&self) -> Result<Db> {
        Db::with_uri(&self.uri).await
    }
}
