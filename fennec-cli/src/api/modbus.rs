pub mod battery;
mod pool;
mod url;
mod value;

use std::{sync::Arc, time::Duration};

use tokio::{sync::Mutex, time::timeout};
use tokio_modbus::client::Reader;

pub use self::{url::ParsedUrl, value::Value};
use crate::{
    api::modbus::url::{DataType, Operation, Register},
    prelude::*,
};

pub struct Client {
    context: Arc<Mutex<tokio_modbus::client::Context>>,
    pub(super) register: Register,
}

impl Client {
    const READ_TIMEOUT: Duration = Duration::from_secs(10);

    #[instrument(
        skip_all,
        fields(address = self.register.address, data_type = ?self.register.options.data_type),
    )]
    pub async fn read(&self) -> Result<Value> {
        let words = self.read_exact(self.register.options.data_type.num_words()).await?;
        match self.register.options.data_type {
            DataType::U16 => Ok(Value::U16(words[0])),
            DataType::I32 =>
            {
                #[expect(clippy::cast_possible_wrap)]
                Ok(Value::I32((u32::from(words[0]) << 16 | u32::from(words[1])) as i32))
            }
            DataType::U64 => Ok(Value::U64(
                u64::from(words[0]) << 48
                    | u64::from(words[1]) << 32
                    | u64::from(words[2]) << 16
                    | u64::from(words[3]),
            )),
        }
    }

    /// Read the exact number of words.
    #[instrument(
        skip_all,
        fields(address = self.register.address, n = n),
    )]
    async fn read_exact(&self, n: u16) -> Result<Vec<u16>> {
        info!("reading…");
        let mut context = self.context.lock().await;
        let read = match self.register.operation {
            Operation::Input => context.read_input_registers(self.register.address, n),
            Operation::Holding => context.read_holding_registers(self.register.address, n),
        };
        let words = timeout(Self::READ_TIMEOUT, read)
            .await
            .context("timeout reading the register")???;
        drop(context);
        ensure!(words.len() == usize::from(n), "read {} words while expected {}", words.len(), n);
        Ok(words)
    }
}
