use crate::{
    bitflags::{CapabilityFlags, StatusFlags},
    connection::ParseBuf,
    error::ProtocolError,
    pool::PoolItem,
};

#[derive(Debug)]
pub struct OkPacket {
    affected_rows: u64,
    last_insert_id: u64,
    status: StatusFlags,
    warnings: u16,
    message: Option<String>,
    session_state_info: Option<String>,
}

impl OkPacket {
    pub fn affected_rows(&self) -> u64 {
        self.affected_rows
    }

    pub fn last_insert_id(&self) -> u64 {
        self.last_insert_id
    }

    pub fn status(&self) -> StatusFlags {
        self.status
    }

    pub fn warnings(&self) -> u16 {
        self.warnings
    }

    pub fn message(&self) -> Option<&String> {
        self.message.as_ref()
    }

    pub fn session_state_info(&self) -> Option<&String> {
        self.session_state_info.as_ref()
    }
}

impl OkPacket {
    pub fn read_ok(packet: &[u8], capabilities: CapabilityFlags) -> Result<Self, ProtocolError> {
        let mut buf = ParseBuf(packet);
        if buf.checked_eat_u8()? != 0x00 {
            return Err(ProtocolError::unexpected_packet(
                buf.0.to_vec(),
                Some("Ok Packet"),
            ));
        }

        let affected_rows = buf.checked_eat_lenenc_int()?;
        let last_insert_id = buf.checked_eat_lenenc_int()?;
        // We assume that CLIENT_PROTOCOL_41 was set
        let status = StatusFlags::from_bits_truncate(buf.checked_eat_u16()?);

        Ok(Self {
            affected_rows,
            last_insert_id,
            status,
            warnings: buf.checked_eat_u16()?,
            message: match buf.checked_eat_lenenc_str() {
                Ok(str) if !str.is_empty() => Some(str.to_owned()),
                _ => None,
            },
            session_state_info: if capabilities.contains(CapabilityFlags::SESSION_TRACK)
                && status.contains(StatusFlags::SESSION_STATE_CHANGED)
            {
                match buf.checked_eat_lenenc_str() {
                    Ok(str) if !str.is_empty() => Some(str.to_owned()),
                    _ => None,
                }
            } else {
                None
            },
        })
    }

    pub fn read_eof(
        packet: PoolItem<'_, Vec<u8>>,
        capabilities: CapabilityFlags,
    ) -> Result<Self, ProtocolError> {
        let mut buf = ParseBuf(&packet);
        // We need to skip affected_rows and insert_id here
        // because valid content of EOF packet includes
        // packet marker, server status and warning count only.
        // (see `read_ok_ex` in sql-common/client.cc)
        let _ = buf.checked_eat_lenenc_int();
        let _ = buf.checked_eat_lenenc_int();

        // We assume that CLIENT_PROTOCOL_41 was set
        let status = StatusFlags::from_bits_truncate(buf.checked_eat_u16()?);

        Ok(Self {
            affected_rows: 0,
            last_insert_id: 0,
            status,
            warnings: buf.checked_eat_u16()?,
            message: match buf.checked_eat_lenenc_str() {
                Ok(str) if !str.is_empty() => Some(str.to_owned()),
                _ => None,
            },
            session_state_info: if capabilities.contains(CapabilityFlags::SESSION_TRACK)
                && status.contains(StatusFlags::SESSION_STATE_CHANGED)
            {
                match buf.checked_eat_lenenc_str() {
                    Ok(str) if !str.is_empty() => Some(str.to_owned()),
                    _ => None,
                }
            } else {
                None
            },
        })
    }

    pub fn read_old_eof(
        packet: PoolItem<'_, Vec<u8>>,
        _capabilities: CapabilityFlags,
    ) -> Result<Self, ProtocolError> {
        let mut buf = ParseBuf(&packet);
        // We assume that CLIENT_PROTOCOL_41 was set
        let warnings = buf.checked_eat_u16()?;
        let status = StatusFlags::from_bits_truncate(buf.checked_eat_u16()?);

        Ok(Self {
            affected_rows: 0,
            last_insert_id: 0,
            status,
            warnings,
            message: None,
            session_state_info: None,
        })
    }
}
