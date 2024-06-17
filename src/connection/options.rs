use {
    super::types::AuthPlugin,
    crate::{bitflags::CapabilityFlags, Stream},
    std::{fmt, time::Duration},
};

pub struct ConnectionOptions<T: Stream> {
    pub user: String,
    pub password: String,
    pub db_name: Option<String>,
    pub connection: T::Options,
    pub max_allowed_packet: Option<usize>,
    pub timeout: Duration,
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

impl<T: Stream> Default for ConnectionOptions<T> {
    fn default() -> Self {
        Self {
            user: String::new(),
            password: String::new(),
            db_name: None,
            connection: Default::default(),
            max_allowed_packet: None,
            timeout: Duration::from_secs(10),
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

impl<T: Stream> fmt::Debug for ConnectionOptions<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("ConnectionOptions");
        debug
            .field("user", &self.user)
            .field("password", &self.password)
            .field("db_name", &self.db_name)
            .field("connection", &self.connection)
            .field("max_allowed_packet", &self.max_allowed_packet)
            .field("timeout", &self.timeout)
            .field("allow_cleartext_password", &self.allow_cleartext_password)
            .field("auth_plugin", &self.auth_plugin);
        #[cfg(feature = "caching-sha2-password")]
        debug.field("server_key", &self.server_key);
        debug.finish()
    }
}

impl<T: Stream> ConnectionOptions<T> {
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
