# `fennec-modbus`

[![docs.rs](https://img.shields.io/docsrs/fennec-modbus?style=for-the-badge)](https://docs.rs/fennec-modbus)
[![Build status](https://img.shields.io/github/actions/workflow/status/eigenein/fennec/check.yaml?style=for-the-badge)](https://github.com/eigenein/fennec/actions/workflows/check.yaml)
[![Activity](https://img.shields.io/github/commit-activity/y/eigenein/fennec?style=for-the-badge)](https://github.com/eigenein/fennec/commits/main/)

🦊 Modular [Modbus](https://www.modbus.org) client.

- **The TCP layer is sans-IO.** Default implementation for Tokio is provided, and may be used with any TCP client.
- **The Modbus layer is sans-IO.** The TCP layer is provided, and the underlying protocol can be used over any transport.
- **Extensible functions.** Most used standard Modbus functions are provided, and the client can use any user-implemented function with arbitrary `BinWrite` arguments and `BinRead` output.

## Sneak peek

```rust,no_run
use anyhow::Result;

use fennec_modbus::client::AsyncClient;
use fennec_modbus::tcp::UnitId;
use fennec_modbus::tcp::tokio::Client;

# #[tokio::main]
# async fn main() -> Result<()> {
let unit_id = UnitId::try_from(1)?;
let client = Client::builder()
    .endpoint("battery.iot.home.arpa:502")
    .build();
let voltage = client
    .read_holding_registers_value::<u16>(unit_id, 39201)
    .await?;
# Ok(())
# }
```

## Command-line Interface

## Disclaimer

The package is used in a live application, but at this point, the public interface is not stabilized and may change wildly.

## Specifications

- [Application protocol specification v1.1b3](https://www.modbus.org/file/secure/modbusprotocolspecification.pdf)
- [Messaging on TCP/IP Implementation Guide](https://www.modbus.org/file/secure/messagingimplementationguide.pdf)
