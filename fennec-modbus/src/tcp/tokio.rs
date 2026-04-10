use alloc::sync::Arc;

use tokio::sync::Mutex;

use crate::tcp::decoder::TransportHeaderDecoder;

/// Modbus TCP client for [`tokio`].
#[derive(Clone)]
pub struct Client {
    socket: Arc<tokio::net::TcpStream>,
    context: Arc<Mutex<TransportHeaderDecoder>>,
}

impl Client {
    pub async fn call<S, R>(&self, request: &S) -> Result<R, Error> {
        todo!()
    }

    async fn poll(&self) {}
}

#[derive(Debug, Error)]
pub enum Error {}
