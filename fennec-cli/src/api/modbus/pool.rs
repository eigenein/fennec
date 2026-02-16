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
use tokio_modbus::{Slave, client::tcp::attach_slave};
use url::Host;

use crate::{
    api::{
        modbus,
        modbus::{Client, url::Endpoint},
    },
    prelude::*,
};

static POOL: Mutex<HashMap<Endpoint, Arc<Mutex<tokio_modbus::client::Context>>, FxBuildHasher>> =
    Mutex::const_new(HashMap::with_hasher(FxBuildHasher));

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Connect to the register by the URL in the form of `modbus+tcp://host:port/slave-id#register`.
#[instrument(
    skip_all,
    fields(host = %url.endpoint.host, port = url.endpoint.port, slave_id = url.endpoint.slave_id),
)]
pub async fn connect(url: &modbus::ParsedUrl) -> Result<Client> {
    let mut pool = POOL.lock().await;
    let context = match pool.entry(url.endpoint.clone()) {
        Entry::Occupied(entry) => entry.get().clone(),
        Entry::Vacant(entry) => {
            let context = Arc::new(Mutex::const_new(new_context(&url.endpoint).await?));
            entry.insert(context).clone()
        }
    };
    drop(pool);
    Ok(Client { context, register: url.register })
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
    info!("connected");
    Ok(tcp_stream)
}
