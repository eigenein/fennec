# `fennec-modbus`

[![docs.rs](https://img.shields.io/docsrs/fennec-modbus?style=for-the-badge)](https://docs.rs/fennec-modbus)
[![Build status](https://img.shields.io/github/actions/workflow/status/eigenein/fennec/check.yaml?style=for-the-badge)](https://github.com/eigenein/fennec/actions/workflows/check.yaml)
[![Activity](https://img.shields.io/github/commit-activity/y/eigenein/fennec?style=for-the-badge)](https://github.com/eigenein/fennec/commits/main/)

🦊 Modular opinionated type-safe [Modbus](https://www.modbus.org) client.

- **The TCP layer is sans-IO.** Default implementation for Tokio is provided, and may be used with any TCP client.
- **The Modbus layer is sans-IO.** The TCP layer is provided, and the underlying protocol can be used over any transport.
- **Extensible functions.** Most used standard Modbus functions are provided, and the client is free to implement custom functions with custom arguments and output.

## Disclaimer

The package is used in a live application, but at this point, the implementation is incomplete, and the public interface is not stabilized and may change wildly.

## Specifications

- [Application protocol specification v1.1b3](https://www.modbus.org/file/secure/modbusprotocolspecification.pdf)
- [Messaging on TCP/IP Implementation Guide](https://www.modbus.org/file/secure/messagingimplementationguide.pdf)
