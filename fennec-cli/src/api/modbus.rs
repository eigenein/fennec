mod battery_state;
mod endpoint;
mod pool;
mod url;
mod value;

use std::{sync::Arc, time::Duration};

use tokio::{sync::Mutex, time::timeout};
use tokio_modbus::client::Reader;

pub use self::{
    battery_state::{BatteryEnergyState, BatterySettings, BatteryState},
    pool::connect,
    url::Url,
};
use crate::{
    api::modbus::{
        url::{Register, RegisterType},
        value::Value,
    },
    prelude::*,
};

pub struct Client {
    context: Arc<Mutex<tokio_modbus::client::Context>>,
    register: Register,
}

impl Client {
    const READ_TIMEOUT: Duration = Duration::from_secs(10);

    pub async fn read<V: Value + Into<T>, T>(&self) -> Result<T> {
        V::read_from(self).await.map(Into::into)
    }

    /// Read the exact number of words.
    #[instrument(
        skip_all,
        fields(address = self.register.address, n = n),
    )]
    pub(super) async fn read_exact(&self, n: u16) -> Result<Vec<u16>> {
        info!("reading…");
        let mut context = self.context.lock().await;
        let read = match self.register.r#type {
            RegisterType::Input => context.read_input_registers(self.register.address, n),
            RegisterType::Holding => context.read_holding_registers(self.register.address, n),
        };
        let words = timeout(Self::READ_TIMEOUT, read)
            .await
            .context("timeout reading the register")???;
        drop(context);
        ensure!(words.len() == usize::from(n), "read {} words while expected {}", words.len(), n);
        Ok(words)
    }
}
