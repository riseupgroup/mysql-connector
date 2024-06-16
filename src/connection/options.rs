use {
    super::types::AuthPlugin,
    crate::bitflags::CapabilityFlags,
    std::{fmt, time::Duration},
};

pub struct ConnectionOptions {
    pub user: String,
    pub password: String,
    pub db_name: Option<String>,
    pub host: Option<String>,
    pub port: u16,
    pub max_allowed_packet: Option<usize>,
    pub timeout: Duration,
    pub nodelay: bool,
    pub allow_cleartext_password: bool,
    /// Ignore auth plugin specified in handshake and start authentication using this plugin.
    pub auth_plugin: Option<AuthPlugin>,
    #[cfg(feature = "caching-sha2-password")]
    #[cfg_attr(doc, doc(cfg(feature = "caching-sha2-password")))]
    pub server_key: Option<std::sync::Arc<crate::PublicKey>>,
    #[cfg(not(feature = "time"))]
    #[cfg_attr(doc, doc(cfg(feature = "time")))]
    pub sleep: Option<&'static dyn Fn(Duration) -> crate::TimeoutFuture>,
}

impl Default for ConnectionOptions {
    fn default() -> Self {
        Self {
            user: String::new(),
            password: String::new(),
            db_name: None,
            host: None,
            port: 3306,
            max_allowed_packet: None,
            timeout: Duration::from_secs(10),
            nodelay: true,
            allow_cleartext_password: false,
            #[cfg(feature = "caching-sha2-password")]
            auth_plugin: Some(AuthPlugin::Sha2),
            #[cfg(not(feature = "caching-sha2-password"))]
            auth_plugin: None,
            #[cfg(feature = "caching-sha2-password")]
            server_key: None,
            #[cfg(not(feature = "time"))]
            sleep: None,
        }
    }
}

impl fmt::Debug for ConnectionOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("ConnectionOptions");
        debug
            .field("user", &self.user)
            .field("password", &self.password)
            .field("db_name", &self.db_name)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("max_allowed_packet", &self.max_allowed_packet)
            .field("timeout", &self.timeout)
            .field("nodelay", &self.nodelay)
            .field("allow_cleartext_password", &self.allow_cleartext_password)
            .field("auth_plugin", &self.auth_plugin);
        #[cfg(feature = "caching-sha2-password")]
        debug.field("server_key", &self.server_key);
        debug.finish()
    }
}

impl ConnectionOptions {
    pub fn get_capabilities(&self) -> CapabilityFlags {
        let mut out = CapabilityFlags::PROTOCOL_41
            | CapabilityFlags::SECURE_CONNECTION
            | CapabilityFlags::TRANSACTIONS
            | CapabilityFlags::PS_MULTI_RESULTS
            | CapabilityFlags::DEPRECATE_EOF
            | CapabilityFlags::PLUGIN_AUTH;

        if self.db_name.is_some() {
            out |= CapabilityFlags::CONNECT_WITH_DB;
        }

        out
    }
}
