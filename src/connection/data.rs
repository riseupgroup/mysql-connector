use {super::types::AuthPlugin, crate::bitflags::CapabilityFlags};

#[derive(Debug)]
pub struct ConnectionData {
    pub(super) id: u32,
    pub(super) is_mariadb: bool,
    pub(super) version: (u16, u16, u16),
    pub(super) capabilities: CapabilityFlags,
    pub(super) nonce: Vec<u8>,
    pub(super) auth_plugin: AuthPlugin,
    pub(super) auth_switched: bool,
    pub(super) max_allowed_packet: usize,
    pub(super) wait_timeout: usize,
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

    pub fn wait_timeout(&self) -> usize {
        self.wait_timeout
    }
}
