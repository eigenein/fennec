use reqwest::Client;

use crate::{api::client, prelude::*};

pub struct Api(Client);

impl Api {
    pub fn try_new() -> Result<Self> {
        Ok(Self(client::try_new()?))
    }
}
