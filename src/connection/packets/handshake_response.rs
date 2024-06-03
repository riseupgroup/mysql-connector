use {
    crate::{
        bitflags::CapabilityFlags,
        connection::{types::AuthPlugin, UTF8MB4_GENERAL_CI, UTF8_GENERAL_CI},
        utils::{lenenc_slice_len, BufMutExt},
        Serialize,
    },
    bytes::BufMut,
    std::collections::HashMap,
};

#[derive(Debug, Clone)]
pub struct HandshakeResponse<'a> {
    capabilities: CapabilityFlags,
    max_packet_size: u32,
    collation: u8,
    scramble: &'a [u8],
    scramble_encoding: ScrambleEncoding,
    user: &'a [u8],
    db_name: Option<&'a [u8]>,
    auth_plugin: Option<AuthPlugin>,
    connect_attributes: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
enum ScrambleEncoding {
    LenEnc,
    U8,
    Null,
}

impl Serialize for HandshakeResponse<'_> {
    fn serialize(&self, buf: &mut Vec<u8>) {
        self.capabilities.serialize(buf);
        self.max_packet_size.serialize(buf);
        self.collation.serialize(buf);
        buf.put_slice(&[0; 23]);
        buf.put_null_slice(self.user);

        match self.scramble_encoding {
            ScrambleEncoding::LenEnc => buf.put_lenenc_slice(self.scramble),
            ScrambleEncoding::U8 => buf.put_u8_slice(self.scramble),
            ScrambleEncoding::Null => buf.put_null_slice(self.scramble),
        }

        if let Some(db_name) = &self.db_name {
            buf.put_null_slice(db_name);
        }
        if let Some(auth_plugin) = &self.auth_plugin {
            auth_plugin.serialize(buf);
        }

        if let Some(attrs) = &self.connect_attributes {
            let len = attrs
                .iter()
                .map(|(k, v)| lenenc_slice_len(k.as_bytes()) + lenenc_slice_len(v.as_bytes()))
                .sum::<u64>();
            buf.put_lenenc_int(len);

            for (name, value) in attrs {
                buf.put_lenenc_slice(name.as_bytes());
                buf.put_lenenc_slice(value.as_bytes());
            }
        }
    }
}

impl<'a> HandshakeResponse<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        scramble: &'a [u8],
        server_version: (u16, u16, u16),
        user: &'a [u8],
        db_name: Option<&'a [u8]>,
        auth_plugin: Option<AuthPlugin>,
        mut capabilities: CapabilityFlags,
        connect_attributes: Option<HashMap<String, String>>,
        max_packet_size: u32,
    ) -> Self {
        let scramble_encoding =
            if capabilities.contains(CapabilityFlags::PLUGIN_AUTH_LENENC_CLIENT_DATA) {
                ScrambleEncoding::LenEnc
            } else if capabilities.contains(CapabilityFlags::SECURE_CONNECTION) {
                ScrambleEncoding::U8
            } else {
                ScrambleEncoding::Null
            };

        if db_name.is_some() {
            capabilities.insert(CapabilityFlags::CONNECT_WITH_DB);
        } else {
            capabilities.remove(CapabilityFlags::CONNECT_WITH_DB);
        }

        if auth_plugin.is_some() {
            capabilities.insert(CapabilityFlags::PLUGIN_AUTH);
        } else {
            capabilities.remove(CapabilityFlags::PLUGIN_AUTH);
        }

        if connect_attributes.is_some() {
            capabilities.insert(CapabilityFlags::CONNECT_ATTRS);
        } else {
            capabilities.remove(CapabilityFlags::CONNECT_ATTRS);
        }

        Self {
            scramble,
            scramble_encoding,
            collation: if server_version >= (5, 5, 3) {
                UTF8MB4_GENERAL_CI as u8
            } else {
                UTF8_GENERAL_CI as u8
            },
            user,
            db_name,
            auth_plugin,
            capabilities,
            connect_attributes,
            max_packet_size,
        }
    }
}
