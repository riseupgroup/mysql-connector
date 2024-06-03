use crate::{error::ProtocolError, Deserialize, ParseBuf};

#[derive(Debug)]
pub struct Stmt {
    pub id: u32,
    pub columns_len: u16,
    pub params_len: u16,
    pub warning_count: u16,
}

impl Deserialize<'_> for Stmt {
    const SIZE: Option<usize> = Some(12);
    type Ctx = ();

    fn deserialize(buf: &mut ParseBuf<'_>, _ctx: Self::Ctx) -> Result<Self, ProtocolError> {
        buf.check_len(12)?;
        buf.skip(1);
        Ok(Self {
            id: buf.eat_u32(),
            columns_len: buf.eat_u16(),
            params_len: buf.eat_u16(),
            warning_count: {
                buf.skip(1);
                buf.eat_u16()
            },
        })
    }
}
