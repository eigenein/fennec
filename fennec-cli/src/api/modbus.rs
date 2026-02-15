mod battery_state;
pub mod legacy;
mod pool;

use std::sync::Arc;

use tokio::sync::Mutex;
use tokio_modbus::Address;

use crate::prelude::*;

pub struct Client {
    context: Arc<Mutex<tokio_modbus::client::Context>>,
    register_address: Address,
}
