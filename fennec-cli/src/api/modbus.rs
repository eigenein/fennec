use std::{str::FromStr, time::Duration};

use tokio::{net::TcpStream, time::timeout};
use tokio_modbus::{
    Address,
    Slave,
    client::{Reader, tcp::attach_slave},
};
use url::{Host, Url};

use crate::prelude::*;

pub mod legacy;

/// Modbus client for a single logical value, potentially spanned over multiple registers.
pub struct Client {
    context: tokio_modbus::client::Context,
    register: Address,
}

impl Client {
    const TIMEOUT: Duration = Duration::from_secs(10);

    /// Connect to the Modbus endpoint in the form of: `modbus+tcp://host:port/slave-id#register`.
    #[instrument]
    pub async fn connect(url: Url) -> Result<Self> {
        info!("connecting…");
        if url.scheme() != "modbus+tcp://" {
            bail!("only `modbus+tcp://` is currently supported");
        }
        let host = url.host().context("the URL must contain host")?;
        let port = url.port().unwrap_or(502);
        let slave_id = {
            let slave_id = url
                .path_segments()
                .and_then(|mut segments| segments.next())
                .context("slave ID must be specified")?;
            u8::from_str(slave_id).with_context(|| format!("incorrect slave ID: `{slave_id}`"))?
        };
        let register = Address::from_str(
            url.fragment().context("register must be specified as the fragment")?,
        )?;
        let tcp_stream = {
            let result = match host {
                Host::Domain(domain) => {
                    timeout(Self::TIMEOUT, TcpStream::connect((domain, port))).await
                }
                Host::Ipv4(ip_address) => {
                    timeout(Self::TIMEOUT, TcpStream::connect((ip_address, port))).await
                }
                Host::Ipv6(ip_address) => {
                    timeout(Self::TIMEOUT, TcpStream::connect((ip_address, port))).await
                }
            };
            result
                .context("timed out while connecting to the battery")?
                .context("failed to connect to the battery")?
        };
        tcp_stream.set_nodelay(true)?;
        Ok(Self { context: attach_slave(tcp_stream, Slave(slave_id)), register })
    }

    #[instrument(skip_all, fields(register = self.register))]
    pub async fn read<V: Value + Into<T>, T>(&mut self) -> Result<T> {
        V::read_from(self).await.map(Into::into)
    }
}

pub trait Value: Sized {
    /// Read [`Self`] from the Modbus [`Client`].
    async fn read_from(client: &mut Client) -> Result<Self>;
}

impl Value for u16 {
    async fn read_from(client: &mut Client) -> Result<Self> {
        client
            .context
            .read_holding_registers(client.register, 1)
            .await??
            .pop()
            .with_context(|| format!("nothing is read from the register #{}", client.register))
    }
}
