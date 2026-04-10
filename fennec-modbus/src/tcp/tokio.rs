#![cfg(feature = "tokio")]

use alloc::{sync::Arc, vec, vec::Vec};

use binrw::{BinRead, BinWrite};
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, ToSocketAddrs},
    sync::Mutex,
};
use tracing::debug;

use crate::{protocol::function::read_holding_registers, tcp};

#[must_use]
struct Inner {
    stream: Mutex<TcpStream>,
    encoder: tcp::Encoder,
}

/// Modbus TCP client for [`tokio`].
#[derive(Clone)]
#[must_use]
pub struct Client(Arc<Inner>);

impl Client {
    pub async fn connect(endpoint: impl ToSocketAddrs) -> Result<Self, Error> {
        let stream = TcpStream::connect(endpoint).await?;
        stream.set_nodelay(true)?;
        socket2::SockRef::from(&stream).set_keepalive(true)?;
        Ok(Self(Arc::new(Inner { stream: Mutex::new(stream), encoder: tcp::Encoder::default() })))
    }

    pub async fn read_holding_registers(
        &self,
        unit_id: tcp::UnitId,
        starting_address: u16,
        n_registers: u16,
    ) -> Result<Vec<u16>, Error> {
        let response = self
            .call::<_, read_holding_registers::Response>(
                unit_id,
                &read_holding_registers::Request::builder()
                    .starting_address(starting_address)
                    .n_registers(n_registers)
                    .build()
                    .map_err(tcp::Error::from)?,
            )
            .await?;
        Ok(response.words)
    }

    /// Low-level interface to call a Modbus function.
    ///
    /// The caller is responsible for matching the request and response.
    pub async fn call<S, R>(&self, unit_id: tcp::UnitId, request: &S) -> Result<R, Error>
    where
        S: for<'a> BinWrite<Args<'a> = ()>,
        R: for<'a> BinRead<Args<'a> = ()>,
    {
        let (frame, transaction_id) = self.0.encoder.prepare(unit_id, request)?;
        let mut stream = self.0.stream.lock().await;

        #[cfg(feature = "tracing")]
        debug!(transaction_id, len = frame.len(), "writing frame");
        stream.write_all(&frame).await?;

        let header = loop {
            #[cfg(feature = "tracing")]
            debug!(transaction_id, "awaiting header…");

            let header = tcp::decode_header(&{
                let mut header_bytes = [0; tcp::Header::SIZE];
                stream.read_exact(&mut header_bytes).await?;
                header_bytes
            })?;

            #[cfg(feature = "tracing")]
            debug!(transaction_id = header.transaction_id, "received header");

            if header.transaction_id == transaction_id {
                break header;
            }
        };

        let mut payload_bytes = vec![0; header.payload_length().into()];
        stream.read_exact(&mut payload_bytes).await?;
        drop(stream);
        Ok(tcp::decode_payload::<R>(&payload_bytes)?)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("transport error")]
    Tcp(#[from] tcp::Error),

    #[error("I/O error")]
    Io(#[from] tokio::io::Error),
}
