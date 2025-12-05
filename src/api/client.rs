use std::time::Duration;

use reqwest::Client;

use crate::prelude::*;

/// Build a default client.
pub fn try_new() -> Result<Client> {
    Ok(Client::builder().timeout(Duration::from_secs(10)).build()?)
}
