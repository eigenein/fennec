//! SMA Sunny Boy clients.

use crate::{api::modbus, prelude::*, quantity::energy::KilowattHours};

pub struct Client(pub modbus::Client);

impl Client {
    pub async fn read_total_yield(&self) -> Result<KilowattHours> {
        let watt_hours = u64::try_from(self.0.read_value().await?)?;
        let kilowatt_hours = watt_hours / 1000;

        #[expect(clippy::cast_precision_loss)]
        Ok((kilowatt_hours as f64).into())
    }
}
