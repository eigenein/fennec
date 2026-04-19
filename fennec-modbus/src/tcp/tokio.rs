//! Modbus-over-TCP implementation for [`tokio`].

#![cfg(feature = "tokio")]

use alloc::{vec, vec::Vec};
use core::{fmt::Debug, time::Duration};

use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, ToSocketAddrs},
    sync::{Mutex, MutexGuard},
    time::timeout,
};

use crate::{
    protocol::{Function, Request, Response, codec::Decode},
    tcp,
    tcp::{Header, transaction},
};

#[must_use]
struct Connection<E> {
    endpoint: E,
    connect_timeout: Duration,
    stream: Mutex<Option<TcpStream>>,
}

impl<E> Connection<E> {
    /// Lazily establish a connection when needed and return the TCP stream.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = "debug"))]
    async fn get(&self) -> Result<ConnectionGuard<'_>, Error>
    where
        E: Clone + ToSocketAddrs,
    {
        let mut guard = self.stream.lock().await;

        if guard.is_none() {
            #[cfg(feature = "tracing")]
            tracing::debug!("connecting…");

            let stream =
                timeout(self.connect_timeout, TcpStream::connect(Clone::clone(&self.endpoint)))
                    .await
                    .map_err(Error::ConnectionTimeout)??;
            stream.set_nodelay(true)?;
            socket2::SockRef::from(&stream).set_keepalive(true)?;
            *guard = Some(stream);
        }

        Ok(ConnectionGuard(guard))
    }
}

struct ConnectionGuard<'a>(MutexGuard<'a, Option<TcpStream>>);

impl ConnectionGuard<'_> {
    fn get_mut(&mut self) -> &mut TcpStream {
        self.0.as_mut().unwrap()
    }

    fn invalidate(mut self) {
        *self.0 = None;
    }
}

/// Modbus TCP client for [`tokio`].
///
/// # Example
///
/// ```rust,no_run
/// use anyhow::Result;
/// use fennec_modbus::{
///     protocol::{address, function::ReadHoldingRegisters},
///     tcp::{UnitId, tokio::Client},
/// };
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let unit_id = UnitId::Significant(1);
///     let client = Client::new("battery.iot.home.arpa:502");
///     let decivolts = client.call::<ReadHoldingRegisters<_, u16>>(unit_id, 39201).await?;
///     Ok(())
/// }
/// ```
///
/// # Connection management
///
/// The underlying connection is managed automatically:
///
/// - An initial connection is established on first use.
/// - The connection is dropped on any error, except for response decoding errors – in that case, the connection itself stays healthy.
/// - Connection is re-established upon next use, so it is safe to retry operations via, for example, `backon`.
/// - It is safe to wrap the client in [`alloc::sync::Arc`] and clone it.
///
/// # Pipelining
///
/// - The pipelining is currently *not supported*. The underlying connection stays locked for the entire transaction.
/// - Mismatching transaction responses are *dropped*.
#[must_use]
pub struct Client<E> {
    encoder: transaction::Encoder,
    connection: Connection<E>,
    round_trip_timeout: Duration,
}

impl<E> Client<E> {
    pub fn new(endpoint: E) -> Self {
        Self {
            encoder: transaction::Encoder::default(),
            connection: Connection {
                endpoint,
                connect_timeout: Duration::from_secs(5),
                stream: Mutex::new(None),
            },
            round_trip_timeout: Duration::from_secs(1),
        }
    }

    pub const fn with_connect_timeout(mut self, duration: Duration) -> Self {
        self.connection.connect_timeout = duration;
        self
    }

    pub const fn with_round_trip_timeout(mut self, duration: Duration) -> Self {
        self.round_trip_timeout = duration;
        self
    }
}

impl<E> Client<E>
where
    E: Clone + ToSocketAddrs,
{
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = "trace"))]
    pub async fn call<F: Function>(
        &self,
        unit_id: tcp::UnitId,
        args: impl Into<F::Args>,
    ) -> Result<F::Output, Error> {
        #[cfg(feature = "tracing")]
        tracing::debug!(?unit_id, code = ?F::CODE, "calling function…");

        let mut frame = Vec::new();
        let transaction_id =
            self.encoder.encode(unit_id, &Request::wrap::<F>(args.into()), &mut frame)?;

        let mut connection = self.connection.get().await?;

        let future = async {
            #[cfg(feature = "tracing")]
            tracing::trace!(transaction_id, len = frame.len(), "writing frame…");
            connection.get_mut().write_all(&frame).await?;

            let header = loop {
                #[cfg(feature = "tracing")]
                tracing::trace!(transaction_id, "awaiting header…");

                let header = {
                    let mut header_bytes = [0; tcp::Header::N_BYTES];
                    connection.get_mut().read_exact(&mut header_bytes).await?;
                    Header::decode(&mut header_bytes.as_slice())?
                };

                #[cfg(feature = "tracing")]
                tracing::trace!(transaction_id = header.transaction_id, "received header");

                if header.transaction_id == transaction_id {
                    break header;
                }

                #[cfg(feature = "tracing")]
                tracing::warn!(header.transaction_id, "discarding response");

                let mut discarded_bytes = vec![0; header.payload_length().into()];
                connection.get_mut().read_exact(&mut discarded_bytes).await?;
            };

            let mut payload_bytes = vec![0; header.payload_length().into()];

            #[cfg(feature = "tracing")]
            tracing::trace!(len = header.payload_length(), "reading payload…");

            connection.get_mut().read_exact(&mut payload_bytes).await?;

            Ok::<_, Error>(payload_bytes)
        };

        let payload_bytes = timeout(self.round_trip_timeout, future)
            .await
            .map_err(Error::TransactionTimeout)
            .flatten()
            .inspect_err(|error| {
                #[cfg(feature = "tracing")]
                tracing::debug!("invalidating connection because of error: {error:#}");

                connection.invalidate();
            })?;
        Ok(Response::<F>::decode(&mut payload_bytes.as_slice())?.into_result()?)
    }
}

impl<E> Client<E> {
    /// Disconnect the client.
    ///
    /// Subsequent call will re-establish a connection.
    /// Note that the client normally disconnects automatically on error.
    ///
    /// This operation is idempotent, closing a closed connection is a no-op.
    pub async fn disconnect(&self) {
        *self.connection.stream.lock().await = None;
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("protocol error")]
    Protocol(#[from] crate::Error),

    #[error("I/O error")]
    Io(#[from] tokio::io::Error),

    #[error("timed out connecting")]
    ConnectionTimeout(tokio::time::error::Elapsed),

    #[error("transaction timeout")]
    TransactionTimeout(tokio::time::error::Elapsed),
}
