use std::{sync::Arc, time::Duration};

use tokio::{sync::Mutex, time::timeout};
use tokio_modbus::client::Reader;

use crate::{
    api::modbus::{
        url::{DataType, Operation, Register},
        value::Value,
    },
    prelude::*,
};

pub struct Client {
    pub(super) context: Arc<Mutex<tokio_modbus::client::Context>>,
    pub(super) register: Register,
}

impl Client {
    const READ_TIMEOUT: Duration = Duration::from_secs(10);

    /// Read the associated value.
    #[instrument(
        skip_all,
        fields(address = self.register.address, data_type = ?self.register.options.data_type),
    )]
    pub async fn read_value(&self) -> Result<Value> {
        let value = self.convert(&self.read_words().await?);
        debug!(?value, "read");
        Ok(value)
    }

    /// Read the raw words from the registers.
    async fn read_words(&self) -> Result<Vec<u16>> {
        let n_words = self.register.options.data_type.num_words();
        debug!(n_words, "reading…");
        let mut context = self.context.lock().await;
        let read = match self.register.operation {
            Operation::Input => context.read_input_registers(self.register.address, n_words),
            Operation::Holding => context.read_holding_registers(self.register.address, n_words),
        };
        let words = timeout(Self::READ_TIMEOUT, read)
            .await
            .context("timeout reading the register")???;
        drop(context);
        ensure!(
            words.len() == usize::from(n_words),
            "read {} words while expected {}",
            words.len(),
            n_words,
        );
        Ok(words)
    }

    /// Convert the raw words to the target value.
    fn convert(&self, words: &[u16]) -> Value {
        match self.register.options.data_type {
            DataType::U16 => Value::U16(words[0]),
            DataType::I32 =>
            {
                #[expect(clippy::cast_possible_wrap)]
                Value::I32((u32::from(words[0]) << 16 | u32::from(words[1])) as i32)
            }
            DataType::U64 => Value::U64(
                u64::from(words[0]) << 48
                    | u64::from(words[1]) << 32
                    | u64::from(words[2]) << 16
                    | u64::from(words[3]),
            ),
        }
    }
}
