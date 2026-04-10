/// Modbus TCP client for [`tokio`].
pub struct Client {
    socket: tokio::net::TcpSocket,
}

impl Client {
    pub async fn call<S, R>(&self, request: &S) -> Result<R, Error> {
        todo!()
    }
}

#[derive(Debug, Error)]
pub enum Error {}
