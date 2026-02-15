use crate::{api::modbus::Client, prelude::*};

pub trait Value: Sized {
    async fn read_from(client: &Client) -> Result<Self>;
}

impl Value for u16 {
    #[instrument(skip_all, fields(type = std::any::type_name::<Self>()))]
    async fn read_from(client: &Client) -> Result<Self> {
        Ok(client.read_exact(1).await?[0])
    }
}
