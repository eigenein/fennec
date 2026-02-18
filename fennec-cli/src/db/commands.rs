use std::time::Duration;

use bson::Document;
use serde::Serialize;
use serde_with::serde_as;

use crate::{db::Measurement, prelude::*};

pub fn set_expiration_time<M: Measurement>(expiration_time: Duration) -> Result<Document> {
    #[serde_as]
    #[derive(Serialize)]
    struct Command {
        #[serde(rename = "collMod")]
        pub collection_name: &'static str,

        #[serde_as(as = "serde_with::DurationSeconds<u64>")]
        #[serde(rename = "expireAfterSeconds")]
        pub expiration_time: Duration,
    }

    info!(collection_name = M::COLLECTION_NAME, "setting expiration time…");
    Ok(bson::serialize_to_document(&Command {
        collection_name: M::COLLECTION_NAME,
        expiration_time,
    })?)
}
