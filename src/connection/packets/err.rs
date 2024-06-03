use {
    crate::{
        bitflags::CapabilityFlags,
        connection::{Deserialize, ParseBuf},
        error::ProtocolError,
    },
    std::fmt,
};

#[derive(Debug)]
pub enum ErrPacket {
    Error(ErrorPacket),
    Progress(ProgressPacket),
}

impl Deserialize<'_> for ErrPacket {
    const SIZE: Option<usize> = None;
    type Ctx = CapabilityFlags;

    fn deserialize(buf: &mut ParseBuf<'_>, capabilities: Self::Ctx) -> Result<Self, ProtocolError> {
        buf.check_len(3)?;
        if buf.eat_u8() != 0xFF {
            return Err(ProtocolError::unexpected_packet(
                buf.0.to_vec(),
                Some("Err Packet"),
            ));
        }
        let code = buf.eat_u16();

        if code == 0xFFFF && capabilities.contains(CapabilityFlags::PROGRESS_OBSOLETE) {
            buf.parse(()).map(ErrPacket::Progress)
        } else {
            buf.parse((code, capabilities.contains(CapabilityFlags::PROTOCOL_41)))
                .map(ErrPacket::Error)
        }
    }
}

pub struct ErrorPacket {
    code: u16,
    state: Option<[u8; 5]>,
    message: String,
}

impl ErrorPacket {
    pub fn code(&self) -> u16 {
        self.code
    }

    pub fn state(&self) -> Option<&[u8; 5]> {
        self.state.as_ref()
    }

    pub fn state_str(&self) -> Option<&str> {
        self.state.as_ref().map(|x| unsafe {
            // Safety: state is validated during parsing
            std::str::from_utf8_unchecked(x)
        })
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Debug for ErrorPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut format = f.debug_struct("ErrorPacket");
        format.field("code", &self.code);
        if let Some(state) = self.state_str() {
            format.field("state", &state);
        }
        format.field("message", &self.message);
        format.finish()
    }
}

impl Deserialize<'_> for ErrorPacket {
    const SIZE: Option<usize> = None;
    type Ctx = (u16, bool);

    fn deserialize(
        buf: &mut crate::connection::ParseBuf<'_>,
        (code, protocol_41): Self::Ctx,
    ) -> Result<Self, ProtocolError> {
        let state = if protocol_41 {
            if buf.checked_eat_u8()? != b'#' {
                return Err(ProtocolError::invalid_packet(
                    buf.0.to_vec(),
                    "Err",
                    "missing state",
                ));
            }
            let state = unsafe { *(buf.checked_eat(5)? as *const _ as *const [u8; 5]) };
            std::str::from_utf8(&state)?;
            Some(state)
        } else {
            None
        };
        Ok(ErrorPacket {
            code,
            state,
            message: String::from_utf8(buf.eat_all().to_owned())?,
        })
    }
}

#[derive(Debug)]
pub struct ProgressPacket {
    stage: u8,
    max_stage: u8,
    progress: u32,
    stage_info: Vec<u8>,
}

impl ProgressPacket {
    pub fn stage(&self) -> u8 {
        self.stage
    }

    pub fn max_stage(&self) -> u8 {
        self.max_stage
    }

    pub fn progress(&self) -> u32 {
        self.progress
    }

    pub fn stage_info(&self) -> &Vec<u8> {
        &self.stage_info
    }
}

impl Deserialize<'_> for ProgressPacket {
    const SIZE: Option<usize> = None;
    type Ctx = ();

    fn deserialize(
        buf: &mut crate::connection::ParseBuf<'_>,
        _ctx: Self::Ctx,
    ) -> Result<Self, ProtocolError> {
        buf.check_len(6)?;
        buf.skip(1);
        Ok(ProgressPacket {
            stage: buf.eat_u8(),
            max_stage: buf.eat_u8(),
            progress: buf.eat_u24(),
            stage_info: buf.checked_eat_lenenc_slice()?.to_vec(),
        })
    }
}
