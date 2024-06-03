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
pub mod types;

const MAX_PAYLOAD_LEN: usize = 16_777_215;
const DEFAULT_MAX_ALLOWED_PACKET: usize = 4 * 1024 * 1024;
const DEFAULT_WAIT_TIMEOUT: usize = 28800;

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

pub(super) use {
    command::Command,
    parse_buf::ParseBuf,
    serialization::{Deserialize, Serialize},
};

pub use {
    data::ConnectionData, options::ConnectionOptions, prepared_statement::PreparedStatement,
    result_set::ResultSet,
};

pub struct Connection<T: Socket> {
    socket: T,
    seq_id: u8,
    data: ConnectionData,
    options: Arc<ConnectionOptions>,
    pending_result: bool,
}

impl<T: Socket> Connection<T> {
    pub fn data(&self) -> &ConnectionData {
        &self.data
    }

    pub fn options(&self) -> Arc<ConnectionOptions> {
        self.options.clone()
    }
}

impl<T: Socket> fmt::Debug for Connection<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("seq_id", &self.seq_id)
            .field("data", &self.data)
            .field("options", &self.options)
            .finish()
    }
}

#[allow(async_fn_in_trait)]
pub trait Socket: Sized + AsyncRead + AsyncWrite + Unpin + fmt::Debug {
    async fn connect(host: &str, port: u16, nodelay: bool) -> Result<Self, std::io::Error>;
}

#[cfg(feature = "tcpstream")]
impl Socket for tokio::net::TcpStream {
    async fn connect(host: &str, port: u16, nodelay: bool) -> Result<Self, std::io::Error> {
        let this = Self::connect((host, port)).await?;
        this.set_nodelay(nodelay)?;
        Ok(this)
    }
}
