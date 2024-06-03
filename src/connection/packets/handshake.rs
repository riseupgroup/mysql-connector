use {
    crate::{
        bitflags::{CapabilityFlags, StatusFlags},
        connection::types::{AuthPlugin, HalfInteger},
        error::ProtocolError,
        Deserialize, ParseBuf,
    },
    std::str::FromStr,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HandshakePacket<'a> {
    protocol_version: u8,
    server_version: &'a str,
    connection_id: u32,
    capabilities: CapabilityFlags,
    default_collation: u8,
    status_flags: StatusFlags,
    auth_plugin_data_len: u8,
    nonce: Vec<u8>,
    auth_plugin: Option<AuthPlugin>,
}

impl<'de> Deserialize<'de> for HandshakePacket<'de> {
    const SIZE: Option<usize> = None;
    type Ctx = ();

    fn deserialize(buf: &mut ParseBuf<'de>, _ctx: Self::Ctx) -> Result<Self, ProtocolError> {
        let protocol_version = buf.parse(())?;
        let server_version = buf.eat_null_str()?;

        // includes trailing 10 bytes filler
        buf.check_len(31)?;
        let connection_id = buf.parse_unchecked(())?;
        let mut nonce = Vec::from(&buf.parse_unchecked::<[u8; 8]>(())?);
        buf.skip(1);
        let capabilities_bits_1 = <u32 as HalfInteger>::deserialize_lower(buf)?;
        let default_collation = buf.parse_unchecked(())?;
        let status_flags = buf.parse_unchecked(())?;
        let capabilities_bits_2 = <u32 as HalfInteger>::deserialize_upper(buf)?;
        let auth_plugin_data_len: u8 = buf.parse_unchecked(())?;
        buf.skip(10);

        let capabilities = CapabilityFlags::try_from(capabilities_bits_1 | capabilities_bits_2)?;

        if capabilities.contains(CapabilityFlags::SECURE_CONNECTION) {
            let len = i8::max(13, auth_plugin_data_len as i8 - 8) as usize;
            nonce.extend_from_slice(buf.checked_eat(len)?);
        }

        // Trim zero terminator. Fill with zeroes if nonce
        // is somehow smaller than 20 bytes (this matches the server behavior).
        nonce.resize(20, 0);

        let auth_plugin = if capabilities.contains(CapabilityFlags::PLUGIN_AUTH) {
            Some(AuthPlugin::from_bytes(buf.eat_all())?)
        } else {
            None
        };

        Ok(Self {
            protocol_version,
            server_version,
            connection_id,
            capabilities,
            default_collation,
            status_flags,
            auth_plugin_data_len,
            nonce,
            auth_plugin,
        })
    }
}

#[allow(dead_code)]
impl<'a> HandshakePacket<'a> {
    pub fn protocol_version(&self) -> u8 {
        self.protocol_version
    }

    pub fn server_version(&self) -> &str {
        self.server_version
    }

    pub fn parse_server_version(&self) -> Option<((u16, u16, u16), bool)> {
        let mut server_version = self.server_version;
        fn eat_int(string: &mut &str) -> Option<u16> {
            match string.chars().position(|x| !x.is_alphanumeric()) {
                Some(pos) => {
                    let value = u16::from_str(&string[..pos]).ok();
                    *string = &string[pos + 1..];
                    value
                }
                None => {
                    let value = u16::from_str(string).ok();
                    *string = "";
                    value
                }
            }
        }

        let version = (
            eat_int(&mut server_version)?,
            eat_int(&mut server_version)?,
            eat_int(&mut server_version)?,
        );
        Some((version, server_version.starts_with("MariaDB")))
    }

    pub fn connection_id(&self) -> u32 {
        self.connection_id
    }

    pub fn nonce(&self) -> &Vec<u8> {
        &self.nonce
    }

    pub fn into_nonce(self) -> Vec<u8> {
        self.nonce
    }

    pub fn capabilities(&self) -> CapabilityFlags {
        self.capabilities
    }

    pub fn default_collation(&self) -> u8 {
        self.default_collation
    }

    pub fn status_flags(&self) -> StatusFlags {
        self.status_flags
    }

    pub fn auth_plugin(&self) -> Option<AuthPlugin> {
        self.auth_plugin
    }
}
