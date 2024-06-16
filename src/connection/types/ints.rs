use crate::{error::ProtocolError, Deserialize, ParseBuf, Serialize};

pub trait HalfInteger: Sized {
    #[allow(dead_code)]
    fn serialize_upper(&self, buf: &mut Vec<u8>);
    #[allow(dead_code)]
    fn serialize_lower(&self, buf: &mut Vec<u8>);
    #[allow(dead_code)]
    fn deserialize_upper(buf: &mut ParseBuf<'_>) -> Result<Self, ProtocolError>;
    #[allow(dead_code)]
    fn deserialize_lower(buf: &mut ParseBuf<'_>) -> Result<Self, ProtocolError>;
}

impl HalfInteger for u32 {
    fn serialize_upper(&self, buf: &mut Vec<u8>) {
        u16::serialize(&((self >> 16) as u16), buf);
    }

    fn serialize_lower(&self, buf: &mut Vec<u8>) {
        u16::serialize(&((self & 0x0000_FFFF) as u16), buf);
    }

    fn deserialize_upper(buf: &mut ParseBuf<'_>) -> Result<Self, ProtocolError> {
        u16::deserialize(buf, ()).map(|x| (x as u32) << 16)
    }

    fn deserialize_lower(buf: &mut ParseBuf<'_>) -> Result<Self, ProtocolError> {
        u16::deserialize(buf, ()).map(|x| x as u32)
    }
}
