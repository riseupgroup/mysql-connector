use {
    super::{Error, ParseError},
    crate::types::ValueType,
    std::{convert::Infallible, fmt, io},
};

#[derive(Debug)]
pub enum SerializeError {
    Infallible,
    InvalidValue(ValueType, Box<dyn fmt::Debug>),
    #[cfg(feature = "caching-sha2-password")]
    #[cfg_attr(doc, doc(cfg(feature = "caching-sha2-password")))]
    Encryption(crate::utils::crypt::Error),
}

impl From<Infallible> for SerializeError {
    fn from(_value: Infallible) -> Self {
        Self::Infallible
    }
}

#[cfg(feature = "caching-sha2-password")]
#[cfg_attr(doc, doc(cfg(feature = "caching-sha2-password")))]
impl From<crate::utils::crypt::Error> for SerializeError {
    fn from(value: crate::utils::crypt::Error) -> Self {
        Self::Encryption(value)
    }
}

pub struct UnexpectedPacket(pub Vec<u8>, pub Option<&'static str>);

impl fmt::Debug for UnexpectedPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(expected) = self.1 {
            f.debug_struct("UnexpectedPacket")
                .field("expected", &expected)
                .field("got", &self.0)
                .finish()
        } else {
            f.debug_tuple("UnexpectedPacket").field(&self.0).finish()
        }
    }
}

#[derive(Debug)]
pub struct InvalidPacket {
    pub packet: Vec<u8>,
    pub r#type: &'static str,
    pub error: &'static str,
}

#[derive(Debug)]
pub enum ProtocolError {
    Parse(ParseError),
    Serialize(SerializeError),
    Io(io::Error),
    OutOfSync,
    UnexpectedPacket(UnexpectedPacket),
    InvalidPacket(InvalidPacket),
    UnknownAuthPlugin(Vec<u8>),
}

impl ProtocolError {
    pub fn eof() -> Self {
        Self::Io(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"))
    }

    pub fn unexpected_packet(got: Vec<u8>, expected: Option<&'static str>) -> Self {
        Self::UnexpectedPacket(UnexpectedPacket(got, expected))
    }

    pub fn invalid_packet(packet: Vec<u8>, r#type: &'static str, error: &'static str) -> Self {
        Self::InvalidPacket(InvalidPacket {
            packet,
            r#type,
            error,
        })
    }
}

impl From<ParseError> for ProtocolError {
    fn from(value: ParseError) -> Self {
        ProtocolError::Parse(value)
    }
}

impl From<ParseError> for Error {
    fn from(value: ParseError) -> Self {
        ProtocolError::Parse(value).into()
    }
}

impl From<SerializeError> for ProtocolError {
    fn from(value: SerializeError) -> Self {
        ProtocolError::Serialize(value)
    }
}

impl From<SerializeError> for Error {
    fn from(value: SerializeError) -> Self {
        ProtocolError::Serialize(value).into()
    }
}

impl From<io::Error> for ProtocolError {
    fn from(value: io::Error) -> Self {
        ProtocolError::Io(value)
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        ProtocolError::Io(value).into()
    }
}
