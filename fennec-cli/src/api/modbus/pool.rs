use std::{
    collections::{HashMap, hash_map::Entry},
    sync::Arc,
    time::Duration,
};

use itertools::Itertools;
use rustc_hash::FxBuildHasher;
use tokio::{
    net::{TcpStream, lookup_host},
    sync::Mutex,
    time::timeout,
};
use tokio_modbus::{Slave, SlaveId, client::tcp::attach_slave};
use url::{Host, Url};

use crate::{api::modbus::Client, prelude::*};

static POOL: Mutex<HashMap<Endpoint, Arc<Mutex<tokio_modbus::client::Context>>, FxBuildHasher>> =
    Mutex::const_new(HashMap::with_hasher(FxBuildHasher));

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_PORT: u16 = 502;

/// Connect to the register by the URL in the form of `modbus+tcp://host:port/slave-id#register`.
#[instrument]
pub async fn connect(url: Url) -> Result<Client> {
    ensure!(url.scheme() == "modbus+tcp://", "only `modbus+tcp://` is currently supported");
    let register_address = url
        .fragment()
        .context("the URL fragment must specify a register address")?
        .parse()
        .context("incorrect register address")?;
    let endpoint = Endpoint::try_from(url)?;
    let mut pool = POOL.lock().await;
    let context = match pool.entry(endpoint.clone()) {
        Entry::Occupied(entry) => entry.get().clone(),
        Entry::Vacant(entry) => {
            let context = Arc::new(Mutex::const_new(new_context(&endpoint).await?));
            entry.insert(context.clone());
            context
        }
    };
    drop(pool);
    Ok(Client { context, register_address })
}

async fn new_context(endpoint: &Endpoint) -> Result<tokio_modbus::client::Context> {
    Ok(attach_slave(new_tcp_stream(&endpoint.host, endpoint.port).await?, Slave(endpoint.slave_id)))
}

async fn new_tcp_stream(host: &Host, port: u16) -> Result<TcpStream> {
    info!("connecting…");
    let addresses = match host {
        Host::Domain(domain) => lookup_host((domain.as_str(), port)).await?.collect_vec(),
        Host::Ipv4(ip_address) => lookup_host((*ip_address, port)).await?.collect_vec(),
        Host::Ipv6(ip_address) => lookup_host((*ip_address, port)).await?.collect_vec(),
    };
    let tcp_stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(&*addresses))
        .await
        .context("timed out while connecting to the battery")?
        .context("failed to connect to the battery")?;
    tcp_stream.set_nodelay(true)?;
    Ok(tcp_stream)
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct Endpoint {
    host: Host,
    port: u16,
    slave_id: SlaveId,
}

impl TryFrom<Url> for Endpoint {
    type Error = Error;

    fn try_from(url: Url) -> Result<Self> {
        let host = url.host().context("the URL must contain host")?.to_owned();
        let port = url.port().unwrap_or(DEFAULT_PORT);
        let slave_id = url
            .path_segments()
            .into_iter()
            .flatten()
            .next()
            .context("slave ID must be specified in the first segment")?
            .parse()
            .context("incorrect slave ID")?;
        Ok(Self { host, port, slave_id })
    }
}
