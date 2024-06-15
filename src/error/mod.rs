mod parse;
mod protocol;

use {
    crate::{connection::types::AuthPlugin, packets::ErrPacket},
    std::io,
};

pub use {
    parse::{InvalidFlags, ParseError},
    protocol::{ProtocolError, SerializeError},
};

#[derive(Debug)]
pub struct AuthPluginMismatch {
    pub current: AuthPlugin,
    pub requested: AuthPlugin,
}

#[derive(Debug)]
pub enum RuntimeError {
    ParameterCountMismatch,
    InsecureAuth,
    AuthPluginMismatch(AuthPluginMismatch),
}

impl RuntimeError {
    pub fn auth_plugin_mismatch(current: AuthPlugin, requested: AuthPlugin) -> Self {
        Self::AuthPluginMismatch(AuthPluginMismatch { current, requested })
    }
}

#[derive(Debug)]
pub enum Error {
    Server(ErrPacket),
    Protocol(ProtocolError),
    Runtime(RuntimeError),
}

impl Error {
    pub fn io_invalid_data<T>(err: T) -> Self
    where
        T: std::error::Error + Send + Sync + 'static,
    {
        Self::Protocol(ProtocolError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            err,
        )))
    }
}

impl From<ErrPacket> for Error {
    fn from(value: ErrPacket) -> Self {
        Self::Server(value)
    }
}

impl From<ProtocolError> for Error {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value)
    }
}

impl From<RuntimeError> for Error {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}
