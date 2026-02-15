use crate::{api::modbus::Client, prelude::*};

pub trait Value: Sized {
    async fn read_from(client: &Client) -> Result<Self>;
}

impl Value for u16 {
    async fn read_from(client: &Client) -> Result<Self> {
        Ok(client.read_exact::<1>().await?[0])
    }
}
