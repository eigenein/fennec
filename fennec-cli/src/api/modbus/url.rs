use std::str::FromStr;

use derive_more::FromStr;
use tokio_modbus::Address;

use crate::{api::modbus::endpoint::Endpoint, prelude::*};

#[derive(Clone)]
pub struct Url {
    pub endpoint: Endpoint,
    pub register: Register,
}

impl Url {
    const DEFAULT_PORT: u16 = 502;
}

impl FromStr for Url {
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
        let r#type = match url.fragment() {
            Some(fragment) => fragment.parse()?,
            None => RegisterType::try_from(address)?,
        };
        Ok(Self {
            endpoint: Endpoint { host, port, slave_id },
            register: Register { address, r#type },
        })
    }
}

#[derive(Copy, Clone, Eq, PartialEq, FromStr)]
pub enum RegisterType {
    Input,
    Holding,
}

impl TryFrom<Address> for RegisterType {
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
    pub address: Address,
    pub r#type: RegisterType,
}
