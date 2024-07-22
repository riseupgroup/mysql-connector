# mysql-connector

Simple MySQL connector for Rust that allows exchanging the underlying connection.

## Features

* `tcpstream` (enabled by default): implements the [`Stream`] trait for tokio's [`tokio::net::TcpStream`].
* `caching-sha2-password` (enabled by default): implements the caching SHA-2 pluggable authentication plugin
* `time` (enabled by default): uses [`tokio::time::sleep`] for network timeout.
* `serde`: implements [`serde::Serialize`] and [`serde::Deserialize`] for some types.
