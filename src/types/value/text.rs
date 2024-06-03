use {
    super::{Value, ValueType},
    crate::{
        bitflags::ColumnFlags,
        connection::types::ColumnType,
        error::{ParseError, ProtocolError},
        ParseBuf,
    },
    std::{
        fmt::{self, Write},
        io,
        str::FromStr,
    },
};

fn write_escaped_chars(
    f: &mut fmt::Formatter<'_>,
    chars: impl Iterator<Item = char>,
) -> fmt::Result {
    // See https://dev.mysql.com/doc/refman/8.0/en/string-literals.html for escape sequences
    for char in chars {
        match char {
            '\0' => {
                f.write_char('\\')?;
                f.write_char('0')?;
            }
            '\u{0008}' => {
                // backspace
                f.write_char('\\')?;
                f.write_char('b')?;
            }
            '\n' => {
                f.write_char('\\')?;
                f.write_char('n')?;
            }
            '\r' => {
                f.write_char('\\')?;
                f.write_char('r')?;
            }
            '\t' => {
                f.write_char('\\')?;
                f.write_char('t')?;
            }
            '\u{001A}' => {
                // ASCII 26 (Control+Z)
                f.write_char('\\')?;
                f.write_char('z')?;
            }
            '\'' => {
                f.write_char('\\')?;
                f.write_char('\'')?;
            }
            '"' => {
                f.write_char('\\')?;
                f.write_char('"')?;
            }
            '\\' => {
                f.write_char('\\')?;
                f.write_char('\\')?;
            }
            x => f.write_char(x)?,
        }
    }
    Ok(())
}

/// Escapes and quotes string.
pub struct StringEscape<'a>(pub &'a str);

impl fmt::Display for StringEscape<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('"')?;
        write_escaped_chars(f, self.0.chars())?;
        f.write_char('"')
    }
}

/// Escapes and quotes identifier.
/// Trailing spaces are removed, as this is required for database, table, and column names.
/// Only the characters `\u{0001}` - `\u{FFFF}` are allowed, everything else is removed.
pub struct IdentifierEscape<'a>(pub &'a str);

impl fmt::Display for IdentifierEscape<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // See https://dev.mysql.com/doc/refman/8.0/en/identifiers.html for identifier rules
        f.write_char('`')?;

        fn char_valid(char: char) -> bool {
            ('\u{0001}'..='\u{FFFF}').contains(&char)
        }

        let mut last_char = 0;
        for (i, char) in self.0.chars().enumerate() {
            if !char.is_whitespace() && char_valid(char) {
                last_char = i;
            }
        }

        for (i, char) in self.0.chars().enumerate() {
            if i <= last_char && char_valid(char) {
                if char == '`' {
                    f.write_char('`')?;
                }
                f.write_char(char)?;
            }
        }
        f.write_char('`')
    }
}

/// Removes all characters from identifier that are not allowed if unquoted.
/// Allowed characters are: `0-9`, `a-z`, `A-Z`, `$`, `_`, `\u{0080}` - `\u{FFFF}`.
///
/// NOTE: This does *not* change identifiers such as "int", which are not allowed to be unquoted.
pub struct UnquotedIdentifierEscape<'a>(pub &'a str);

impl fmt::Display for UnquotedIdentifierEscape<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // See https://dev.mysql.com/doc/refman/8.0/en/identifiers.html for identifier rules
        fn char_valid(char: char) -> bool {
            matches!(char, '0'..='9' | 'a'..='z' | 'A'..='Z' | '$' | '_' | '\u{0080}'..='\u{FFFF}')
        }

        for char in self.0.chars() {
            if char_valid(char) {
                f.write_char(char)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn format_date(
            f: &mut fmt::Formatter<'_>,
            year: &u16,
            month: &u8,
            day: &u8,
        ) -> fmt::Result {
            write!(f, "{year:0>4}-{month:0>2}-{day:0>2}")
        }
        fn format_time(
            f: &mut fmt::Formatter<'_>,
            hours: &u32,
            minutes: &u8,
            seconds: &u8,
            microseconds: &u32,
        ) -> fmt::Result {
            write!(f, "{hours:0>2}:{minutes:0>2}:{seconds:0>2}")?;
            if *microseconds != 0 {
                write!(f, ".{microseconds:0>6}")?;
            }
            Ok(())
        }
        match self {
            Value::Null => write!(f, "NULL"),
            Value::Tiny(x) => x.fmt(f),
            Value::Short(x) => x.fmt(f),
            Value::Int(x) => x.fmt(f),
            Value::Long(x) => x.fmt(f),
            Value::UTiny(x) => x.fmt(f),
            Value::UShort(x) => x.fmt(f),
            Value::UInt(x) => x.fmt(f),
            Value::ULong(x) => x.fmt(f),
            Value::Float(x) => x.fmt(f),
            Value::Double(x) => x.fmt(f),
            Value::Bytes(x) => {
                f.write_str("0x")?;
                hex::encode(x).fmt(f)
            }
            Value::Date(year, month, day) => {
                write!(f, "\"")?;
                format_date(f, year, month, day)?;
                write!(f, "\"")
            }
            Value::Time(negative, days, hours, minutes, seconds, microseconds) => {
                write!(f, "\"")?;
                if *negative {
                    write!(f, "-")?;
                }
                format_time(
                    f,
                    &(*days * 24 + *hours as u32),
                    minutes,
                    seconds,
                    microseconds,
                )?;
                write!(f, "\"")
            }
            Value::Datetime(year, month, day, hour, minute, second, microsecond) => {
                write!(f, "\"")?;
                format_date(f, year, month, day)?;
                write!(f, " ")?;
                format_time(f, &(*hour as u32), minute, second, microsecond)?;
                write!(f, "\"")
            }
        }
    }
}

impl Value {
    fn deserialize_time_text_inner(value: &[u8]) -> Result<(u32, u8, u8, u8, u32), ProtocolError> {
        let invalid_value = || ParseError::InvalidValue(ValueType::Time, value.to_vec());
        let first_colon = value
            .iter()
            .position(|x| *x == b':')
            .ok_or(ProtocolError::eof())?;
        if value.len() < first_colon + 6 || value[first_colon + 3] != b':' {
            return Err(invalid_value().into());
        }
        let hours: u32 = btoi::btou(&value[0..first_colon]).map_err(|_| invalid_value())?;
        let minutes: u8 =
            btoi::btou(&value[first_colon + 1..first_colon + 3]).map_err(|_| invalid_value())?;
        let seconds: u8 =
            btoi::btou(&value[first_colon + 4..first_colon + 6]).map_err(|_| invalid_value())?;
        let mut microseconds: u32 = 0;
        if value.len() > first_colon + 6 + 1 {
            if value[first_colon + 6] != b'.' {
                return Err(invalid_value().into());
            }
            let buf = &value[first_colon + 6 + 1..];
            microseconds = btoi::btou(buf).map_err(|_| invalid_value())?;
            microseconds *= 10u32.pow(6 - buf.len() as u32);
        }
        Ok((
            hours / 24,
            (hours % 24) as u8,
            minutes,
            seconds,
            microseconds,
        ))
    }

    fn deserialize_date_text_inner(value: &[u8]) -> Result<(u16, u8, u8), ProtocolError> {
        // possible range for date: 1000-01-01 to 9999-12-31

        let invalid_value = || ParseError::InvalidValue(ValueType::Date, value.to_vec());

        if value.len() != 10 || value[4] != b'-' || value[7] != b'-' {
            return Err(invalid_value().into());
        }

        let year: u16 = btoi::btou(&value[0..4]).map_err(|_| invalid_value())?;
        let month: u8 = btoi::btou(&value[5..7]).map_err(|_| invalid_value())?;
        let day: u8 = btoi::btou(&value[8..10]).map_err(|_| invalid_value())?;
        Ok((year, month, day))
    }

    fn deserialize_time_text(value: &[u8]) -> Result<Self, ProtocolError> {
        if value.is_empty() {
            return Err(ProtocolError::eof());
        }
        let negative = if value[0] == b'-' { 1 } else { 0 };
        let (days, hours, minutes, seconds, microseconds) =
            Self::deserialize_time_text_inner(&value[negative..])?;
        Ok(Self::Time(
            negative == 1,
            days,
            hours,
            minutes,
            seconds,
            microseconds,
        ))
    }

    fn deserialize_date_text(value: &[u8]) -> Result<Self, ProtocolError> {
        Self::deserialize_date_text_inner(value)
            .map(|(year, month, day)| Self::Date(year, month, day))
    }

    fn deserialize_datetime_text(value: &[u8]) -> Result<Self, ProtocolError> {
        // possible range for datetime: 1000-01-01 00:00:00 to 9999-12-31 23:59:59
        // possible range for timestamp: 1970-01-01 00:00:01 to 2038-01-19 03:14:07

        if value.len() < 19 {
            return Err(ParseError::InvalidValue(ValueType::Date, value.to_vec()).into());
        }
        let (year, month, day) = Self::deserialize_date_text_inner(&value[0..10])?;
        let (_, hour, minute, second, microsecond) =
            Self::deserialize_time_text_inner(&value[11..])?;
        Ok(Self::Datetime(
            year,
            month,
            day,
            hour,
            minute,
            second,
            microsecond,
        ))
    }

    pub(crate) fn deserialize_text(
        column_type: ColumnType,
        column_flags: ColumnFlags,
        buf: &mut ParseBuf<'_>,
    ) -> Result<Self, ProtocolError> {
        if buf.is_empty() {
            return Err(ProtocolError::eof());
        }

        match buf.0[0] {
            0xfb => {
                buf.skip(1);
                Ok(Value::Null)
            }
            _ => {
                let bytes = buf.checked_eat_lenenc_slice()?;
                match column_type {
                    ColumnType::Tiny
                    | ColumnType::Short
                    | ColumnType::Year
                    | ColumnType::Long
                    | ColumnType::Int24
                    | ColumnType::LongLong => match column_type {
                        ColumnType::Tiny => {
                            if column_flags.contains(ColumnFlags::UNSIGNED_FLAG) {
                                btoi::btou::<u8>(bytes).map(Value::UTiny)
                            } else {
                                btoi::btoi::<i8>(bytes).map(Value::Tiny)
                            }
                        }
                        ColumnType::Short | ColumnType::Year => {
                            if column_flags.contains(ColumnFlags::UNSIGNED_FLAG) {
                                btoi::btou::<u16>(bytes).map(Value::UShort)
                            } else {
                                btoi::btoi::<i16>(bytes).map(Value::Short)
                            }
                        }
                        ColumnType::Long | ColumnType::Int24 => {
                            if column_flags.contains(ColumnFlags::UNSIGNED_FLAG) {
                                btoi::btou::<u32>(bytes).map(Value::UInt)
                            } else {
                                btoi::btoi::<i32>(bytes).map(Value::Int)
                            }
                        }
                        ColumnType::LongLong => {
                            if column_flags.contains(ColumnFlags::UNSIGNED_FLAG) {
                                btoi::btou::<u64>(bytes).map(Value::ULong)
                            } else {
                                btoi::btoi::<i64>(bytes).map(Value::Long)
                            }
                        }
                        _ => unreachable!(),
                    }
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e).into()),
                    ColumnType::Float | ColumnType::Double => {
                        let text = std::str::from_utf8(bytes)
                            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
                        match column_type {
                            ColumnType::Float => {
                                f32::from_str(text).map(|x| x.into()).map_err(|err| {
                                    io::Error::new(io::ErrorKind::InvalidData, err).into()
                                })
                            }
                            ColumnType::Double => {
                                f64::from_str(text).map(|x| x.into()).map_err(|err| {
                                    io::Error::new(io::ErrorKind::InvalidData, err).into()
                                })
                            }
                            _ => unreachable!(),
                        }
                    }
                    ColumnType::String
                    | ColumnType::VarString
                    | ColumnType::Varchar
                    | ColumnType::Blob
                    | ColumnType::TinyBlob
                    | ColumnType::MediumBlob
                    | ColumnType::Set
                    | ColumnType::Enum
                    | ColumnType::Decimal
                    | ColumnType::Bit
                    | ColumnType::NewDecimal
                    | ColumnType::Geometry
                    | ColumnType::Json => Ok(Self::Bytes(bytes.to_vec())),
                    ColumnType::Timestamp | ColumnType::Datetime => {
                        Self::deserialize_datetime_text(bytes)
                    }
                    ColumnType::Date => Self::deserialize_date_text(bytes),
                    ColumnType::Time => Self::deserialize_time_text(bytes),
                    x => unimplemented!("Unsupported column type {:?}", x),
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use {
        super::{IdentifierEscape, StringEscape, UnquotedIdentifierEscape},
        crate::types::Value,
    };

    #[test]
    fn string_escape() {
        assert_eq!(
            StringEscape(r#"Robert'); DROP TABLE Students;--"#).to_string(),
            r#""Robert\'); DROP TABLE Students;--""#
        );
        assert_eq!(
            StringEscape(r#"" or ""=""#).to_string(),
            r#""\" or \"\"=\"""#
        );
        assert_eq!(
            StringEscape(r#"' or ''='"#).to_string(),
            r#""\' or \'\'=\'""#
        );
        assert_eq!(StringEscape("abc\0def").to_string(), r#""abc\0def""#);
        assert_eq!(StringEscape("abc\u{0008}def").to_string(), r#""abc\bdef""#);
        assert_eq!(StringEscape("abc\ndef").to_string(), r#""abc\ndef""#);
        assert_eq!(StringEscape("abc\rdef").to_string(), r#""abc\rdef""#);
        assert_eq!(StringEscape("abc\tdef").to_string(), r#""abc\tdef""#);
        assert_eq!(StringEscape("abc\u{001A}def").to_string(), r#""abc\zdef""#);
        assert_eq!(StringEscape("abc\'def").to_string(), r#""abc\'def""#);
        assert_eq!(StringEscape("abc\"def").to_string(), r#""abc\"def""#);
        assert_eq!(StringEscape("abc\\def").to_string(), r#""abc\\def""#);
    }

    #[test]
    fn identifier_escape() {
        assert_eq!(IdentifierEscape("id").to_string(), "`id`");
        assert_eq!(IdentifierEscape("i d ").to_string(), "`i d`");
        assert_eq!(IdentifierEscape("äöü").to_string(), "`äöü`");
        assert_eq!(IdentifierEscape("`test`").to_string(), "```test```");
        assert_eq!(IdentifierEscape("\u{0}test\u{10000}").to_string(), "`test`");
    }

    #[test]
    fn unquoted_identifier_escape() {
        assert_eq!(UnquotedIdentifierEscape(r#"test id"#).to_string(), "testid");
        assert_eq!(UnquotedIdentifierEscape(r#"Id0"#).to_string(), "Id0");
        assert_eq!(UnquotedIdentifierEscape(r#"0$_äöü"#).to_string(), "0$_äöü");
        assert_eq!(UnquotedIdentifierEscape(r#"`id`"#).to_string(), "id");
        assert_eq!(UnquotedIdentifierEscape(r#""id""#).to_string(), "id");
        assert_eq!(UnquotedIdentifierEscape(r#"'id'"#).to_string(), "id");
        assert_eq!(UnquotedIdentifierEscape(r#"\bid"#).to_string(), "bid");
    }

    #[test]
    fn display() {
        assert_eq!(Value::Null.to_string(), "NULL");
        assert_eq!(Value::Int(-5).to_string(), "-5");
        assert_eq!(Value::Float(4.5).to_string(), "4.5");
        assert_eq!(
            Value::Bytes(vec![32, 64, 123, 213]).to_string(),
            "0x20407bd5"
        );
        assert_eq!(Value::Date(2038, 01, 19).to_string(), "\"2038-01-19\"");
        assert_eq!(
            Value::Time(false, 0, 4, 5, 8, 0).to_string(),
            "\"04:05:08\""
        );
        assert_eq!(
            Value::Time(false, 6, 4, 15, 18, 50).to_string(),
            "\"148:15:18.000050\""
        );
        assert_eq!(
            Value::Datetime(2038, 01, 19, 3, 14, 7, 0).to_string(),
            "\"2038-01-19 03:14:07\""
        );
        assert_eq!(
            Value::Datetime(2038, 01, 19, 3, 14, 7, 50).to_string(),
            "\"2038-01-19 03:14:07.000050\""
        );
    }

    #[test]
    fn parse() {
        assert_eq!(
            Value::deserialize_date_text("2038-01-19".as_bytes()).unwrap(),
            Value::Date(2038, 1, 19)
        );
        assert_eq!(
            Value::deserialize_time_text("04:05:08".as_bytes()).unwrap(),
            Value::Time(false, 0, 4, 5, 8, 0)
        );
        assert_eq!(
            Value::deserialize_time_text("148:15:18.000050".as_bytes()).unwrap(),
            Value::Time(false, 6, 4, 15, 18, 50)
        );
        assert_eq!(
            Value::deserialize_datetime_text("2038-01-19 03:14:07".as_bytes()).unwrap(),
            Value::Datetime(2038, 01, 19, 3, 14, 7, 0)
        );
        assert_eq!(
            Value::deserialize_datetime_text("2038-01-19 03:14:07.000050".as_bytes()).unwrap(),
            Value::Datetime(2038, 01, 19, 3, 14, 7, 50)
        );
    }
}
