use {super::types::AuthPlugin, crate::bitflags::CapabilityFlags};

#[derive(Debug)]
pub struct ConnectionOptions {
    pub user: String,
    pub password: String,
    pub db_name: Option<String>,
    pub host: String,
    pub port: u16,
    pub max_allowed_packet: Option<usize>,
    pub wait_timeout: Option<usize>,
    pub nodelay: bool,
    pub allow_cleartext_password: bool,
    /// Only allow [`AuthPlugin`]::Sha2 and [`AuthPlugin`]::Native authentication.
    pub secure_auth: bool,
    /// Ignore auth plugin specified in handshake and start authentication using this plugin.
    pub auth_plugin: Option<AuthPlugin>,
}

impl Default for ConnectionOptions {
    fn default() -> Self {
        Self {
            user: Default::default(),
            password: Default::default(),
            db_name: None,
            host: String::from("localhost"),
            port: 3306,
            max_allowed_packet: None,
            wait_timeout: None,
            nodelay: true,
            allow_cleartext_password: false,
            secure_auth: true,
            auth_plugin: None,
        }
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
