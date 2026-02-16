//! SMA Sunny Boy clients.

use crate::{api::modbus, prelude::*, quantity::energy::KilowattHours};

pub struct Client(pub modbus::Client);

impl Client {
    pub async fn read_total_export(&self) -> Result<KilowattHours> {
        let watt_hours = f64::try_from(self.0.read_value().await?)?;
        Ok((watt_hours / 1000.0).into())
    }
}
