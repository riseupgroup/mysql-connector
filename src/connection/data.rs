use {
    super::types::AuthPlugin,
    crate::{bitflags::CapabilityFlags, TimeoutFuture},
    std::fmt,
};

pub struct ConnectionData {
    pub(super) id: u32,
    pub(super) is_mariadb: bool,
    pub(super) version: (u16, u16, u16),
    pub(super) capabilities: CapabilityFlags,
    pub(super) nonce: Vec<u8>,
    #[cfg(feature = "caching-sha2-password")]
    #[cfg_attr(doc, doc(cfg(feature = "caching-sha2-password")))]
    pub(super) server_key: Option<std::sync::Arc<crate::PublicKey>>,
    pub(super) auth_plugin: AuthPlugin,
    pub(super) auth_switched: bool,
    pub(super) max_allowed_packet: usize,
    pub(super) sleep: &'static (dyn Fn(std::time::Duration) -> TimeoutFuture + Send + Sync),
}

impl ConnectionData {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn is_mariadb(&self) -> bool {
        self.is_mariadb
    }

    pub fn version(&self) -> (u16, u16, u16) {
        self.version
    }

    pub fn capabilities(&self) -> CapabilityFlags {
        self.capabilities
    }

    pub fn auth_plugin(&self) -> AuthPlugin {
        self.auth_plugin
    }

    pub fn max_allowed_packet(&self) -> usize {
        self.max_allowed_packet
    }
}

impl fmt::Debug for ConnectionData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("ConnectionData");
        debug
            .field("id", &self.id)
            .field("is_mariadb", &self.is_mariadb)
            .field("version", &self.version)
            .field("capabilities", &self.capabilities)
            .field("nonce", &self.nonce);
        #[cfg(feature = "caching-sha2-password")]
        debug.field("server_key", &self.server_key);
        debug
            .field("auth_plugin", &self.auth_plugin)
            .field("auth_switched", &self.auth_switched)
            .field("max_allowed_packet", &self.max_allowed_packet)
            .finish()
    }
}
