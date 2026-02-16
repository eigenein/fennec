use std::str::FromStr;

use derive_more::FromStr;
use serde::Deserialize;
use tokio_modbus::{Address, SlaveId};
use url::Host;

use crate::{
    api::modbus::{Client, Value, pool::connect},
    prelude::*,
};

#[derive(Clone)]
pub struct ParsedUrl {
    pub(super) endpoint: Endpoint,
    pub(super) register: Register,
}

impl ParsedUrl {
    const DEFAULT_PORT: u16 = 502;

    pub async fn connect(&self) -> Result<Client> {
        connect(self).await
    }

    pub async fn read(&self) -> Result<Value> {
        self.connect().await?.read().await
    }
}

impl FromStr for ParsedUrl {
    type Err = Error;

    fn from_str(url: &str) -> Result<Self, Self::Err> {
        let url = url::Url::from_str(url).with_context(|| format!("`{url}` is an invalid URL"))?;
        ensure!(url.scheme() == "modbus+tcp", "only `modbus+tcp` scheme is currently supported");
        let host = url.host().context("the URL must contain host")?.to_owned();
        let port = url.port().unwrap_or(Self::DEFAULT_PORT);
        let mut path_segments = url.path_segments().into_iter().flatten();
        let slave_id = path_segments
            .next()
            .context("slave ID must be specified in the first segment")?
            .parse()
            .context("incorrect slave ID")?;
        let address = path_segments
            .next()
            .context("register address must be specified in the second segment")?
            .parse()
            .context("incorrect register address")?;
        let operation = match url.fragment() {
            Some(fragment) => fragment.parse()?,
            None => Operation::try_from(address)?,
        };
        let options = match url.query() {
            Some(query) => serde_qs::from_str(query)?,
            None => Options::default(),
        };
        Ok(Self {
            endpoint: Endpoint { host, port, slave_id },
            register: Register { address, operation, options },
        })
    }
}

/// Modbus slave connection endpoint.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Endpoint {
    pub host: Host,
    pub port: u16,
    pub slave_id: SlaveId,
}

#[derive(Copy, Clone, Eq, PartialEq, FromStr)]
pub enum Operation {
    Input,
    Holding,
}

impl TryFrom<Address> for Operation {
    type Error = Error;

    fn try_from(address: Address) -> std::result::Result<Self, Self::Error> {
        match address {
            30000..=39999 => Ok(Self::Input),
            40000..=49999 => Ok(Self::Holding),
            _ => bail!("cannot determine register #{address} type – specify explicitly"),
        }
    }
}

#[derive(Copy, Clone)]
pub struct Register {
    pub(super) address: Address,
    pub(super) operation: Operation,
    pub(super) options: Options,
}

#[derive(Copy, Clone, Default, Deserialize)]
pub(super) struct Options {
    #[serde(rename = "type")]
    pub data_type: DataType,
}

#[derive(Copy, Clone, Debug, Default, Deserialize)]
pub enum DataType {
    #[default]
    #[serde(rename = "u16")]
    U16,

    #[serde(rename = "i32")]
    I32,

    #[serde(rename = "u64")]
    U64,
}

impl DataType {
    pub const fn num_words(self) -> u16 {
        match self {
            Self::U16 => 1,
            Self::I32 => 2,
            Self::U64 => 4,
        }
    }
}
