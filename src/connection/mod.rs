mod auth;
pub(super) mod bitflags;
mod command;
mod data;
mod init;
mod io;
mod options;
pub(super) mod packets;
mod parse_buf;
mod prepared_statement;
mod query;
mod result_set;
mod serialization;
pub mod timeout;
pub mod types;

const MAX_PAYLOAD_LEN: usize = 16_777_215;
const DEFAULT_MAX_ALLOWED_PACKET: usize = 4 * 1024 * 1024;

const UTF8_GENERAL_CI: u16 = 33;
const UTF8MB4_GENERAL_CI: u16 = 45;

lazy_static::lazy_static! {
    static ref BUFFER_POOL: SyncPool<Vec<u8>, 64> = SyncPool::new(VecPoolCtx {
        size_cap: DEFAULT_MAX_ALLOWED_PACKET,
        init_size: 1024,
    });
}

use {
    crate::pool::{SyncPool, VecPoolCtx},
    std::{fmt, sync::Arc},
    tokio::io::{AsyncRead, AsyncWrite},
};

pub(crate) use {
    command::Command,
    parse_buf::ParseBuf,
    serialization::{Deserialize, Serialize},
};

pub use {
    data::ConnectionData,
    options::ConnectionOptions,
    prepared_statement::PreparedStatement,
    result_set::ResultSet,
    timeout::{Timeout, TimeoutFuture},
};

pub struct Connection<T: Stream> {
    stream: T,
    seq_id: u8,
    data: ConnectionData,
    options: Arc<ConnectionOptions>,
    pending_result: bool,
}

impl<T: Stream> Connection<T> {
    pub fn data(&self) -> &ConnectionData {
        &self.data
    }

    pub fn options(&self) -> Arc<ConnectionOptions> {
        self.options.clone()
    }
}

impl<T: Stream> fmt::Debug for Connection<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("seq_id", &self.seq_id)
            .field("data", &self.data)
            .field("options", &self.options)
            .finish()
    }
}

#[allow(async_fn_in_trait)]
pub trait Stream: Sized + AsyncRead + AsyncWrite + Unpin + fmt::Debug {
    /// Set this to `true` if the connection is a socket or a shared-memory connection.
    const SECURE: bool;

    async fn connect(host: &str, port: u16, nodelay: bool) -> Result<Self, std::io::Error>;
}

#[cfg(feature = "tcpstream")]
#[cfg_attr(doc, doc(cfg(feature = "tcpstream")))]
impl Stream for tokio::net::TcpStream {
    const SECURE: bool = false;

    async fn connect(host: &str, port: u16, nodelay: bool) -> Result<Self, std::io::Error> {
        let this = Self::connect((host, port)).await?;
        this.set_nodelay(nodelay)?;
        Ok(this)
    }
}
