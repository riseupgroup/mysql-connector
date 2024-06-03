macro_rules! column_type {
    ($($name:ident = $value:literal,)*) => {
        #[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
        #[repr(u8)]
        pub enum ColumnType {
            $($name = $value,)*
        }

        impl TryFrom<u8> for ColumnType {
            type Error = crate::error::InvalidFlags;

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value {
                    $($value => Ok(Self::$name),)*
                    _ => Err(crate::error::InvalidFlags::ColumnType(value)),
                }
            }
        }
    };
}

column_type! {
    Decimal = 0,
    Tiny = 1,
    Short = 2,
    Long = 3,
    Float = 4,
    Double = 5,
    Null = 6,
    Timestamp = 7,
    LongLong = 8,
    Int24 = 9,
    Date = 10,
    Time = 11,
    Datetime = 12,
    Year = 13,
    NewDate = 14, // Internal to MySql
    Varchar = 15,
    Bit = 16,
    Timestamp2 = 17,
    Datetime2 = 18,
    Time2 = 19,
    TypedArray = 20, // Used for replication only
    Unknown = 243,
    Json = 245,
    NewDecimal = 246,
    Enum = 247,
    Set = 248,
    TinyBlob = 249,
    MediumBlob = 250,
    LongBlob = 251,
    Blob = 252,
    VarString = 253,
    String = 254,
    Geometry = 255,
}
