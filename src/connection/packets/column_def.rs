use crate::{
    bitflags::ColumnFlags, connection::types::ColumnType, error::ProtocolError, Deserialize,
    ParseBuf,
};

#[derive(Debug)]
#[allow(dead_code)]
pub struct ColumnDef<'a> {
    pub(crate) db: &'a str,
    pub(crate) table: &'a str,
    pub(crate) org_table: &'a str,
    pub(crate) name: &'a str,
    pub(crate) org_name: &'a str,
    pub(crate) charset: u16,
    pub(crate) length: u32,
    pub(crate) r#type: ColumnType,
    pub(crate) flags: ColumnFlags,
    pub(crate) decimals: u8,
}

impl<'de> Deserialize<'de> for ColumnDef<'de> {
    const SIZE: Option<usize> = None;
    type Ctx = ();

    fn deserialize(buf: &mut ParseBuf<'de>, _ctx: Self::Ctx) -> Result<Self, ProtocolError> {
        if buf.checked_eat_u8_slice()? != b"def" {
            return Err(ProtocolError::unexpected_packet(
                buf.0.to_vec(),
                Some("Column Definition"),
            ));
        }
        let db = buf.checked_eat_u8_str()?;
        let table = buf.checked_eat_u8_str()?;
        let org_table = buf.checked_eat_u8_str()?;
        let name = buf.checked_eat_u8_str()?;
        let org_name = buf.checked_eat_u8_str()?;
        buf.check_len(13)?;
        if buf.eat_u8() != 12 {
            return Err(ProtocolError::eof());
        }
        let res = Ok(Self {
            db,
            table,
            org_table,
            name,
            org_name,
            charset: buf.eat_u16(),
            length: buf.eat_u32(),
            r#type: ColumnType::try_from(buf.eat_u8())?,
            flags: ColumnFlags::try_from(buf.eat_u16())?,
            decimals: buf.eat_u8(),
        });
        buf.skip(2);
        res
    }
}
