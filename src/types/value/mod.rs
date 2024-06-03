mod bin;
mod conversion;
mod text;

pub(crate) use conversion::impl_try_into_option;
pub use text::{IdentifierEscape, StringEscape, UnquotedIdentifierEscape};

use crate::connection::types::ColumnType;

/// Mysql value.
///
/// # Formatting
/// Values of type `Date`, `Time` and `DateTime` will be put in quotation marks.
/// Values of type `Bytes` will be converted to hex.
/// If you only want to escape strings, use [`StringEscape`] instead.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Tiny(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    UTiny(u8),
    UShort(u16),
    UInt(u32),
    ULong(u64),
    Float(f32),
    Double(f64),
    Bytes(Vec<u8>),
    /// year, month, day
    Date(u16, u8, u8),
    /// is negative, days, hours, minutes, seconds, micro seconds
    Time(bool, u32, u8, u8, u8, u32),
    /// year, month, day, hour, minute, second, micro second
    Datetime(u16, u8, u8, u8, u8, u8, u32),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ValueType {
    Number,
    Tiny,
    Short,
    Int,
    Long,
    UTiny,
    UShort,
    UInt,
    ULong,
    Float,
    Double,
    Bytes,
    Date,
    Time,
    Datetime,
}

impl Value {
    pub fn column_type(&self) -> ColumnType {
        match self {
            Self::Null => ColumnType::Null,
            Self::Tiny(_) | Value::UTiny(_) => ColumnType::Tiny,
            Self::Short(_) | Value::UShort(_) => ColumnType::Short,
            Self::Int(_) | Value::UInt(_) => ColumnType::Long,
            Self::Long(_) | Value::ULong(_) => ColumnType::LongLong,
            Self::Float(_) => ColumnType::Float,
            Self::Double(_) => ColumnType::Double,
            Self::Bytes(_) => ColumnType::VarString,
            Self::Date(..) => ColumnType::Date,
            Self::Time(..) => ColumnType::Time,
            Self::Datetime(..) => ColumnType::Datetime,
        }
    }

    pub fn is_unsigned(&self) -> bool {
        matches!(
            self,
            Self::UTiny(_) | Self::UShort(_) | Self::UInt(_) | Self::ULong(_)
        )
    }

    pub fn take(&mut self) -> Self {
        std::mem::replace(self, Self::Null)
    }
}

pub trait SimpleValue {
    fn value(&self) -> &Value;
}

impl SimpleValue for Value {
    fn value(&self) -> &Value {
        self
    }
}
