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
    crate::pool::{AsyncPoolContent, AsyncPoolContentError, SyncPool, VecPoolCtx},
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
    options::{ConnectionOptions, ConnectionOptionsTrait},
    prepared_statement::PreparedStatement,
    result_set::ResultSet,
    timeout::{Timeout, TimeoutFuture},
};

pub struct Connection {
    stream: Box<dyn StreamRequirements>,
    seq_id: u8,
    data: ConnectionData,
    options: Arc<dyn ConnectionOptionsTrait>,
    pending_result: bool,
}

impl Connection {
    pub fn data(&self) -> &ConnectionData {
        &self.data
    }

    pub fn options(&self) -> Arc<dyn ConnectionOptionsTrait> {
        self.options.clone()
    }
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("seq_id", &self.seq_id)
            .field("data", &self.data)
            .field("options", &self.options)
            .finish()
    }
}

pub trait StreamRequirements:
    AsyncRead + AsyncWrite + Unpin + fmt::Debug + Send + Sync + 'static
{
}
impl<T: AsyncRead + AsyncWrite + Unpin + fmt::Debug + Send + Sync + 'static> StreamRequirements
    for T
{
}

#[allow(async_fn_in_trait)]
pub trait Stream: Sized + StreamRequirements {
    /// Set this to `true` if the connection is a socket or a shared-memory connection.
    const SECURE: bool;
    type Options: Default + fmt::Debug + Send + Sync;

    async fn connect(data: &Self::Options) -> Result<Self, std::io::Error>;
}

impl AsyncPoolContentError for Connection {
    type Error = crate::Error;
}

impl<T: Stream> AsyncPoolContent<T> for Connection {
    type Ctx = Arc<ConnectionOptions<T>>;

    fn new<'a>(
        ctx: &'a Self::Ctx,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>> + 'a>> {
        Box::pin(Self::connect(Arc::clone(ctx)))
    }
}

#[cfg(feature = "tcpstream")]
#[cfg_attr(doc, doc(cfg(feature = "tcpstream")))]
#[derive(Debug)]
pub struct TcpStreamOptions {
    pub host: String,
    pub port: u16,
    pub nodelay: bool,
}

#[cfg(feature = "tcpstream")]
#[cfg_attr(doc, doc(cfg(feature = "tcpstream")))]
impl Default for TcpStreamOptions {
    fn default() -> Self {
        Self {
            host: String::from("localhost"),
            port: 3306,
            nodelay: true,
        }
    }
}

#[cfg(feature = "tcpstream")]
#[cfg_attr(doc, doc(cfg(feature = "tcpstream")))]
impl Stream for tokio::net::TcpStream {
    const SECURE: bool = false;
    type Options = TcpStreamOptions;

    async fn connect(data: &Self::Options) -> Result<Self, std::io::Error> {
        let this = Self::connect((data.host.as_str(), data.port)).await?;
        this.set_nodelay(data.nodelay)?;
        Ok(this)
    }
}
