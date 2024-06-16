#![cfg_attr(doc, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

pub mod connection;
pub mod error;
pub mod migrator;
pub mod model;
pub mod pool;
pub mod types;
mod utils;

pub use {connection::*, error::Error, mysql_connector_macros as macros};

#[cfg(feature = "caching-sha2-password")]
#[cfg_attr(doc, doc(cfg(feature = "caching-sha2-password")))]
pub use utils::crypt::PublicKey;

#[cfg(feature = "tcpstream")]
#[cfg_attr(doc, doc(cfg(feature = "tcpstream")))]
pub use tokio::net::TcpStream;
