mod battery_state;
pub mod legacy;
mod pool;
mod value;

use std::{sync::Arc, time::Duration};

use tokio::{sync::Mutex, time::timeout};
use tokio_modbus::{Address, client::Reader};

use crate::{api::modbus::value::Value, prelude::*};

pub struct Client {
    context: Arc<Mutex<tokio_modbus::client::Context>>,
    register_address: Address,
}

impl Client {
    const READ_TIMEOUT: Duration = Duration::from_secs(10);

    pub async fn read<V: Value + Into<T>, T>(&self) -> Result<T> {
        V::read_from(self).await.map(Into::into)
    }

    /// Read the exact number of words.
    pub(super) async fn read_exact<const N: usize>(&self) -> Result<[u16; N]> {
        let n = u16::try_from(N)?;
        let mut context = self.context.lock().await;
        let read = match self.register_address {
            30000..=39999 => context.read_input_registers(self.register_address, n),
            40000..=49999 => context.read_holding_registers(self.register_address, n),
            _ => bail!("cannot read words from register #{}", self.register_address),
        };
        let words = timeout(Self::READ_TIMEOUT, read)
            .await
            .context("timeout reading the register")???;
        drop(context);
        words
            .try_into()
            .map_err(|words: Vec<u16>| anyhow!("read {} words while expected {}", words.len(), N))
    }
}
