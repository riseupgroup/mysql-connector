use {
    super::Value,
    crate::{
        bitflags::ColumnFlags,
        connection::types::ColumnType,
        error::ProtocolError,
        utils::{lenenc_slice_len, BufMutExt},
        ParseBuf, Serialize,
    },
    bytes::BufMut,
};

impl Value {
    pub fn bin_len(&self) -> u64 {
        match self {
            Self::Null => 0,
            Self::Tiny(_) => 1,
            Self::Short(_) => 2,
            Self::Int(_) => 4,
            Self::Long(_) => 8,
            Self::UTiny(_) => 1,
            Self::UShort(_) => 2,
            Self::UInt(_) => 4,
            Self::ULong(_) => 8,
            Self::Float(_) => 4,
            Self::Double(_) => 8,
            Self::Bytes(x) => lenenc_slice_len(x),
            Self::Date(0, 0, 0) | Self::Datetime(0, 0, 0, 0, 0, 0, 0) => 1,
            Self::Date(_, _, _) | Self::Datetime(_, _, _, 0, 0, 0, 0) => 5,
            Self::Datetime(_, _, _, _, _, _, 0) => 8,
            Self::Datetime(_, _, _, _, _, _, _) => 12,
            Self::Time(_, 0, 0, 0, 0, 0) => 1,
            Self::Time(_, _, _, _, _, 0) => 9,
            Self::Time(_, _, _, _, _, _) => 13,
        }
    }
}

impl Serialize for Value {
    fn serialize(&self, buf: &mut Vec<u8>) {
        match self {
            Self::Null => (),
            Self::Tiny(x) => buf.put_i8(*x),
            Self::Short(x) => buf.put_i16_le(*x),
            Self::Int(x) => buf.put_i32_le(*x),
            Self::Long(x) => buf.put_i64_le(*x),
            Self::UTiny(x) => buf.put_u8(*x),
            Self::UShort(x) => buf.put_u16_le(*x),
            Self::UInt(x) => buf.put_u32_le(*x),
            Self::ULong(x) => buf.put_u64_le(*x),
            Self::Float(x) => buf.put_f32_le(*x),
            Self::Double(x) => buf.put_f64_le(*x),
            Self::Bytes(x) => buf.put_lenenc_slice(x),
            Self::Datetime(0, 0, 0, 0, 0, 0, 0) => {
                buf.put_u8(0);
            }
            Self::Date(year, month, day) | Self::Datetime(year, month, day, 0, 0, 0, 0) => {
                buf.put_u8(4);
                buf.put_u16_le(*year);
                buf.put_u8(*month);
                buf.put_u8(*day);
            }
            Self::Datetime(year, mon, day, hour, min, sec, 0) => {
                buf.put_u8(7);
                buf.put_u16_le(*year);
                buf.put_u8(*mon);
                buf.put_u8(*day);
                buf.put_u8(*hour);
                buf.put_u8(*min);
                buf.put_u8(*sec);
            }
            Self::Datetime(year, mon, day, hour, min, sec, usec) => {
                buf.put_u8(11);
                buf.put_u16_le(*year);
                buf.put_u8(*mon);
                buf.put_u8(*day);
                buf.put_u8(*hour);
                buf.put_u8(*min);
                buf.put_u8(*sec);
                buf.put_u32_le(*usec);
            }
            Self::Time(_, 0, 0, 0, 0, 0) => {
                buf.put_u8(0);
            }
            Self::Time(neg, d, h, m, s, 0) => {
                buf.put_u8(8);
                buf.put_u8(if *neg { 1 } else { 0 });
                buf.put_u32_le(*d);
                buf.put_u8(*h);
                buf.put_u8(*m);
                buf.put_u8(*s);
            }
            Self::Time(neg, days, hours, mins, secs, usecs) => {
                buf.put_u8(12);
                buf.put_u8(if *neg { 1 } else { 0 });
                buf.put_u32_le(*days);
                buf.put_u8(*hours);
                buf.put_u8(*mins);
                buf.put_u8(*secs);
                buf.put_u32_le(*usecs);
            }
        }
    }
}

macro_rules! deserialize_num {
    ($i:ty, $u:ty, $name:ident) => {
        paste::paste! {
            fn [< deserialize_ $name >](buf: &mut ParseBuf<'_>, unsigned: bool) -> Result<Self, ProtocolError> {
                if unsigned {
                    buf.[< checked_eat_ $u >]().map(|x| x.into())
                } else {
                    buf.[< checked_eat_ $i >]().map(|x| x.into())
                }
            }
        }
    };
}

impl Value {
    deserialize_num!(i8, u8, tiny);
    deserialize_num!(i16, u16, short);
    deserialize_num!(i32, u32, int);
    deserialize_num!(i64, u64, long);

    fn deserialize_date(buf: &mut ParseBuf<'_>) -> Result<Self, ProtocolError> {
        let len = buf.checked_eat_u8()?;
        buf.check_len(len as usize)?;

        let mut year = 0u16;
        let mut month = 0u8;
        let mut day = 0u8;

        if len >= 4 {
            year = buf.eat_u16();
            month = buf.eat_u8();
            day = buf.eat_u8();
        }

        Ok(Self::Date(year, month, day))
    }

    fn deserialize_datetime(buf: &mut ParseBuf<'_>) -> Result<Self, ProtocolError> {
        let len = buf.checked_eat_u8()?;
        buf.check_len(len as usize)?;

        let mut year = 0u16;
        let mut month = 0u8;
        let mut day = 0u8;
        let mut hour = 0u8;
        let mut minute = 0u8;
        let mut second = 0u8;
        let mut micro_second = 0u32;

        if len >= 4 {
            year = buf.eat_u16();
            month = buf.eat_u8();
            day = buf.eat_u8();
        }
        if len >= 7 {
            hour = buf.eat_u8();
            minute = buf.eat_u8();
            second = buf.eat_u8();
        }
        if len == 11 {
            micro_second = buf.eat_u32();
        }

        Ok(Self::Datetime(
            year,
            month,
            day,
            hour,
            minute,
            second,
            micro_second,
        ))
    }

    fn deserialize_time(buf: &mut ParseBuf<'_>) -> Result<Self, ProtocolError> {
        let len = buf.checked_eat_u8()?;
        buf.check_len(len as usize)?;

        let mut is_negative = false;
        let mut days = 0u32;
        let mut hours = 0u8;
        let mut minutes = 0u8;
        let mut seconds = 0u8;
        let mut micro_seconds = 0u32;

        if len >= 8 {
            is_negative = buf.eat_u8() == 1;
            days = buf.eat_u32();
            hours = buf.eat_u8();
            minutes = buf.eat_u8();
            seconds = buf.eat_u8();
        }
        if len == 12 {
            micro_seconds = buf.eat_u32();
        }

        Ok(Self::Time(
            is_negative,
            days,
            hours,
            minutes,
            seconds,
            micro_seconds,
        ))
    }

    pub(crate) fn deserialize_bin(
        column_type: ColumnType,
        column_flags: ColumnFlags,
        buf: &mut ParseBuf<'_>,
    ) -> Result<Self, ProtocolError> {
        match column_type {
            ColumnType::Tiny => {
                Self::deserialize_tiny(buf, column_flags.contains(ColumnFlags::UNSIGNED_FLAG))
            }
            ColumnType::Short | ColumnType::Year => {
                Self::deserialize_short(buf, column_flags.contains(ColumnFlags::UNSIGNED_FLAG))
            }
            ColumnType::Long | ColumnType::Int24 => {
                Self::deserialize_int(buf, column_flags.contains(ColumnFlags::UNSIGNED_FLAG))
            }
            ColumnType::LongLong => {
                Self::deserialize_long(buf, column_flags.contains(ColumnFlags::UNSIGNED_FLAG))
            }
            ColumnType::Float => buf.checked_eat_f32().map(Self::Float),
            ColumnType::Double => buf.checked_eat_f64().map(Self::Double),
            ColumnType::String
            | ColumnType::VarString
            | ColumnType::Varchar
            | ColumnType::Blob
            | ColumnType::TinyBlob
            | ColumnType::MediumBlob
            | ColumnType::LongBlob
            | ColumnType::Set
            | ColumnType::Enum
            | ColumnType::Decimal
            | ColumnType::Bit
            | ColumnType::NewDecimal
            | ColumnType::Geometry
            | ColumnType::Json => buf
                .checked_eat_lenenc_slice()
                .map(|x| Self::Bytes(x.to_vec())),
            ColumnType::Date => Self::deserialize_date(buf),
            ColumnType::Timestamp | ColumnType::Datetime => Self::deserialize_datetime(buf),
            ColumnType::Time => Self::deserialize_time(buf),
            ColumnType::Null => Ok(Self::Null),
            x => unimplemented!("Unsupported column type {:?}", x),
        }
    }
}
