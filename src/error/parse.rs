use {
    super::ProtocolError,
    crate::types::{Value, ValueType},
    std::{fmt, str::Utf8Error, string::FromUtf8Error},
};

pub struct WrongValue(pub ValueType, pub Value);

impl fmt::Debug for WrongValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WrongValue")
            .field("expected", &self.0)
            .field("got", &self.1)
            .finish()
    }
}

#[derive(Debug)]
pub enum InvalidFlags {
    Status(u16),
    Capability(u32),
    CursorType(u8),
    StmtExecuteParams(u8),
    StmtExecuteParam(u8),
    Column(u16),
    ColumnType(u8),
}

#[derive(Debug)]
pub enum ParseError {
    MissingField(&'static str),
    RowLengthMismatch,
    WrongValue(WrongValue),
    InvalidValue(ValueType, Vec<u8>),
    ValueOutOfBounds(Value),
    UnknownBitflags(InvalidFlags),
    Utf8(Utf8Error),
    FromUtf8(FromUtf8Error),
}

impl ParseError {
    pub fn wrong_value(expected: ValueType, got: Value) -> Self {
        Self::WrongValue(WrongValue(expected, got))
    }
}

impl From<InvalidFlags> for ParseError {
    fn from(value: InvalidFlags) -> Self {
        ParseError::UnknownBitflags(value)
    }
}

impl From<InvalidFlags> for ProtocolError {
    fn from(value: InvalidFlags) -> Self {
        ParseError::UnknownBitflags(value).into()
    }
}

impl From<Utf8Error> for ParseError {
    fn from(value: Utf8Error) -> Self {
        ParseError::Utf8(value)
    }
}

impl From<Utf8Error> for ProtocolError {
    fn from(value: Utf8Error) -> Self {
        ParseError::Utf8(value).into()
    }
}

impl From<FromUtf8Error> for ParseError {
    fn from(value: FromUtf8Error) -> Self {
        ParseError::FromUtf8(value)
    }
}

impl From<FromUtf8Error> for ProtocolError {
    fn from(value: FromUtf8Error) -> Self {
        ParseError::FromUtf8(value).into()
    }
}
