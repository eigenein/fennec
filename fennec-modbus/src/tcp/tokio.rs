//! Modbus-over-TCP implementation for [`tokio`].

#![cfg(feature = "tokio")]

use alloc::{vec, vec::Vec};
use core::{fmt::Debug, time::Duration};

use bon::bon;
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, ToSocketAddrs},
    sync::{Mutex, MutexGuard},
    time::timeout,
};

use crate::{
    protocol,
    protocol::{
        Function,
        data_unit,
        function::{ReadHoldingRegisters, ReadHoldingRegistersExact, read_registers},
        r#struct::Readable,
    },
    tcp,
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
/// use fennec_modbus::tcp::{UnitId, tokio::Client};
///
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// let unit_id = UnitId::try_from(1)?;
/// let client = Client::builder().endpoint("battery.iot.home.arpa:502").build();
/// let voltage = client.read_holding_registers_exact::<1>(unit_id, 39201).await?[0];
/// # Ok(())
/// # }
/// ```
///
/// # Connection management
///
/// The underlying connection is managed automatically:
///
/// - An initial connection is established on first use.
/// - The connection is dropped on any error, except for response decoding errors – in that case, the connection itself stays healthy.
///
/// # Pipelining
///
/// - The pipelining is currently *not supported*.
/// - Mismatching transactions are *dropped*.
#[must_use]
pub struct Client<E> {
    encoder: tcp::Encoder,
    connection: Connection<E>,
    round_trip_timeout: Duration,
}

#[bon]
impl<E> Client<E> {
    #[builder]
    pub fn new(
        /// Connection endpoint, anything that supports [`tokio::net::ToSocketAddrs`].
        endpoint: E,
        /// Timeout for establishing a connection.
        #[builder(default = Duration::from_secs(5))]
        connect_timeout: Duration,
        /// Round-trip timeout for entire function call.
        #[builder(default = Duration::from_secs(1))]
        round_trip_timeout: Duration,
    ) -> Self {
        Self {
            encoder: tcp::Encoder::default(),
            connection: Connection { endpoint, connect_timeout, stream: Mutex::new(None) },
            round_trip_timeout,
        }
    }
}

impl<E> Client<E>
where
    E: Clone + ToSocketAddrs,
{
    /// Disconnect the client.
    ///
    /// Subsequent call will re-establish a connection.
    /// Note that the client normally disconnects automatically on error.
    ///
    /// This operation is idempotent, closing a closed connection is a no-op.
    pub async fn disconnect(&self) {
        *self.connection.stream.lock().await = None;
    }

    /// Read the contents of a contiguous block of holding registers in a remote device.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = "trace"))]
    pub async fn read_holding_registers(
        &self,
        unit_id: tcp::UnitId,
        starting_address: u16,
        n_registers: u16,
    ) -> Result<Vec<u16>, Error> {
        #[cfg(feature = "tracing")]
        tracing::trace!(?unit_id, starting_address, n_registers, "reading holding registers…");

        let args = read_registers::Args::builder()
            .starting_address(starting_address)
            .n_registers(n_registers)
            .build()?;
        Ok(self.call::<ReadHoldingRegisters>(unit_id, args).await?.words)
    }

    /// Read the contents of a contiguous block of holding registers in a remote device.
    ///
    /// This is the same function as [`Self::read_holding_registers`] – but with the register count known at compile time.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = "trace"))]
    pub async fn read_holding_registers_exact<const N: usize>(
        &self,
        unit_id: tcp::UnitId,
        starting_address: u16,
    ) -> Result<[u16; N], Error> {
        #[cfg(feature = "tracing")]
        tracing::trace!(?unit_id, starting_address, N, "reading holding registers…");

        let args = read_registers::ArgsExact::<N>::new(starting_address);
        Ok(self.call::<ReadHoldingRegistersExact<N>>(unit_id, args).await?.words)
    }

    /// Call the Modbus function.
    ///
    /// This is a lower-level interface that allows calling any [`Function`], including user ones.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = "trace"))]
    pub async fn call<F: Function>(
        &self,
        unit_id: tcp::UnitId,
        args: F::Args,
    ) -> Result<F::Output, Error> {
        #[cfg(feature = "tracing")]
        tracing::debug!(?unit_id, code = ?F::CODE, "calling function…");

        let (frame, transaction_id) =
            self.encoder.wrap(unit_id, &data_unit::Request::wrap::<F>(args))?;
        let mut connection = self.connection.get().await?;

        let future = async {
            #[cfg(feature = "tracing")]
            tracing::trace!(transaction_id, len = frame.len(), "writing frame");
            connection.get_mut().write_all(&frame).await?;

            let header = loop {
                #[cfg(feature = "tracing")]
                tracing::trace!(transaction_id, "awaiting header…");

                let header = tcp::Header::from_bytes(&{
                    let mut header_bytes = [0; tcp::Header::SIZE];
                    connection.get_mut().read_exact(&mut header_bytes).await?;
                    header_bytes
                })?;

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
        Ok(data_unit::Response::<F>::from_bytes(&payload_bytes)?.into_result()?)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("TCP transport error")]
    Tcp(#[from] tcp::Error),

    #[error("I/O error")]
    Io(#[from] tokio::io::Error),

    #[error("timed out connecting")]
    ConnectionTimeout(tokio::time::error::Elapsed),

    #[error("transaction timeout")]
    TransactionTimeout(tokio::time::error::Elapsed),
}

impl From<protocol::Error> for Error {
    fn from(error: protocol::Error) -> Self {
        Self::Tcp(tcp::Error::Protocol(error))
    }
}
