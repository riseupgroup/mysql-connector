use {
    crate::{
        connection::types::AuthPlugin, error::ProtocolError, Deserialize, ParseBuf, Serialize,
    },
    bytes::BufMut,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AuthSwitchRequest {
    plugin: AuthPlugin,
    data: Vec<u8>,
}

impl AuthSwitchRequest {
    pub fn plugin(&self) -> AuthPlugin {
        self.plugin
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }
}

impl<'de> Deserialize<'de> for AuthSwitchRequest {
    const SIZE: Option<usize> = None;
    type Ctx = ();

    fn deserialize(buf: &mut ParseBuf<'de>, _ctx: Self::Ctx) -> Result<Self, ProtocolError> {
        buf.skip(1);
        Ok(Self {
            plugin: buf.parse(())?,
            data: match buf.eat_all() {
                [head @ .., 0] => head.to_vec(),
                all => all.to_vec(),
            },
        })
    }
}

impl Serialize for AuthSwitchRequest {
    fn serialize(&self, buf: &mut Vec<u8>) {
        buf.put_u8(0x00);
        self.plugin.serialize(buf);
        buf.put_slice(&self.data);
    }
}
