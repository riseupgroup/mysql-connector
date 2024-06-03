use {
    super::{Value, ValueType},
    crate::error::{ParseError, SerializeError},
    chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike},
};

macro_rules! impl_conversion {
    ($t:ty, $name:ident) => {
        impl From<$t> for Value {
            fn from(value: $t) -> Self {
                Value::$name(value)
            }
        }

        impl TryInto<$t> for Value {
            type Error = ParseError;

            fn try_into(self) -> std::result::Result<$t, Self::Error> {
                match self {
                    Value::$name(x) => Ok(x),
                    _ => Err(Self::Error::wrong_value(ValueType::$name, self)),
                }
            }
        }
    };
}

impl_conversion!(i8, Tiny);
impl_conversion!(i16, Short);
impl_conversion!(i32, Int);
impl_conversion!(i64, Long);

impl_conversion!(u8, UTiny);
impl_conversion!(u16, UShort);
impl_conversion!(u32, UInt);
impl_conversion!(u64, ULong);

impl_conversion!(f32, Float);
impl_conversion!(f64, Double);

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Tiny(if value { 1 } else { 0 })
    }
}

impl TryInto<bool> for Value {
    type Error = ParseError;

    fn try_into(self) -> Result<bool, Self::Error> {
        match self {
            Value::Tiny(x) => Ok(x > 0),
            Value::Short(x) => Ok(x > 0),
            Value::Int(x) => Ok(x > 0),
            Value::Long(x) => Ok(x > 0),
            Value::UTiny(x) => Ok(x > 0),
            Value::UShort(x) => Ok(x > 0),
            Value::UInt(x) => Ok(x > 0),
            Value::ULong(x) => Ok(x > 0),
            _ => Err(Self::Error::wrong_value(ValueType::Number, self.clone())),
        }
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::Bytes(value.into_bytes())
    }
}

impl TryInto<String> for Value {
    type Error = ParseError;

    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            Value::Bytes(x) => String::from_utf8(x).map_err(Into::into),
            _ => Err(Self::Error::wrong_value(ValueType::Bytes, self)),
        }
    }
}

impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Value::Bytes(value)
    }
}

impl TryInto<Vec<u8>> for Value {
    type Error = ParseError;

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        match self {
            Value::Bytes(x) => Ok(x),
            _ => Err(Self::Error::wrong_value(ValueType::Bytes, self)),
        }
    }
}

impl TryFrom<NaiveDate> for Value {
    type Error = SerializeError;

    fn try_from(value: NaiveDate) -> Result<Self, Self::Error> {
        Ok(Value::Date(
            value
                .year()
                .try_into()
                .map_err(|_| SerializeError::InvalidValue(ValueType::Date, Box::new(value)))?,
            value.month() as u8,
            value.day() as u8,
        ))
    }
}

impl TryFrom<NaiveDateTime> for Value {
    type Error = SerializeError;

    fn try_from(value: NaiveDateTime) -> Result<Self, Self::Error> {
        Ok(Value::Datetime(
            value
                .year()
                .try_into()
                .map_err(|_| SerializeError::InvalidValue(ValueType::Datetime, Box::new(value)))?,
            value.month() as u8,
            value.day() as u8,
            value.hour() as u8,
            value.minute() as u8,
            value.second() as u8,
            value.nanosecond() / 1_000,
        ))
    }
}

impl TryInto<NaiveDate> for Value {
    type Error = ParseError;

    fn try_into(self) -> Result<NaiveDate, Self::Error> {
        match self {
            Value::Date(year, month, day) | Value::Datetime(year, month, day, _, _, _, _) => {
                NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
                    .ok_or(ParseError::ValueOutOfBounds(self.clone()))
            }
            _ => Err(Self::Error::wrong_value(ValueType::Date, self)),
        }
    }
}

impl TryInto<NaiveDateTime> for Value {
    type Error = ParseError;

    fn try_into(self) -> Result<NaiveDateTime, Self::Error> {
        match self {
            Value::Datetime(year, month, day, hour, minute, second, micro) => {
                let date = match NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32) {
                    Some(x) => x,
                    None => return Err(ParseError::ValueOutOfBounds(self)),
                };
                let time =
                    NaiveTime::from_hms_micro_opt(hour as u32, minute as u32, second as u32, micro)
                        .ok_or(ParseError::ValueOutOfBounds(self))?;
                Ok(NaiveDateTime::new(date, time))
            }
            _ => Err(Self::Error::wrong_value(ValueType::Datetime, self)),
        }
    }
}

impl From<Duration> for Value {
    fn from(value: Duration) -> Self {
        let seconds = value.num_seconds().abs();
        let minutes = seconds / 60;
        let hours = minutes / 60;
        let days = hours / 24;

        Value::Time(
            value.num_seconds() < 0,
            days as u32,
            (hours % 24) as u8,
            (minutes % 60) as u8,
            (seconds % 60) as u8,
            value.subsec_nanos().unsigned_abs() / 1_000,
        )
    }
}

impl TryInto<Duration> for Value {
    type Error = ParseError;

    fn try_into(self) -> Result<Duration, Self::Error> {
        match self {
            Value::Time(negative, days, hours, minutes, seconds, micros) => {
                const MICROS_PER_SEC: u32 = 1_000_000;
                if micros >= MICROS_PER_SEC {
                    return Err(ParseError::ValueOutOfBounds(self.clone()));
                }
                let mut x = days as i64;
                x *= 24;
                x += hours as i64;
                x *= 60;
                x += minutes as i64;
                x *= 60;
                x += seconds as i64;

                // Duration is saved as seconds plus positive shift in nanoseconds.
                // This means, -5.2 seconds becomes -6 seconds plus 800_000_000 nanoseconds.
                let nanos = if negative {
                    x *= -1;

                    if micros != 0 {
                        x -= 1;
                        (MICROS_PER_SEC - micros) * 1_000
                    } else {
                        0
                    }
                } else {
                    micros * 1_000
                };

                Duration::new(x, nanos).ok_or_else(|| ParseError::ValueOutOfBounds(self.clone()))
            }
            _ => Err(Self::Error::wrong_value(ValueType::Time, self.clone())),
        }
    }
}

impl<T> TryFrom<Option<T>> for Value
where
    T: TryInto<Value>,
{
    type Error = <T as TryInto<Value>>::Error;

    fn try_from(value: Option<T>) -> Result<Self, Self::Error> {
        match value {
            Some(value) => value.try_into(),
            None => Ok(Self::Null),
        }
    }
}

macro_rules! impl_try_into_option {
    ($($t:ty),* $(,)?) => {
        $(
            impl TryInto<Option<$t>> for $crate::types::Value {
                type Error = $crate::error::ParseError;

                fn try_into(self) -> Result<Option<$t>, Self::Error> {
                    match self {
                        $crate::types::Value::Null => Ok(None),
                        x => x.try_into().map(Some),
                    }
                }
            }
        )*
    };
}
pub(crate) use impl_try_into_option;

impl_try_into_option!(i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, bool);
impl_try_into_option!(String, NaiveDate, NaiveDateTime, Duration, Vec<u8>);
