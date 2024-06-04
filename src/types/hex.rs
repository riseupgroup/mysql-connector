use {
    super::{value::impl_try_into_option, Value, ValueType},
    crate::error::ParseError,
    std::{fmt, ops},
};

#[derive(Debug, PartialEq, Eq)]
pub struct Hex(pub Vec<u8>);

impl Hex {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    pub fn try_from_hex<T: AsRef<[u8]>>(data: T) -> Result<Self, hex::FromHexError> {
        let mut data: &[u8] = data.as_ref();
        if data[..2] == *b"0x" || data[..2] == *b"0X" {
            data = &data[2..];
        }
        hex::decode(data).map(Self)
    }
}

impl std::str::FromStr for Hex {
    type Err = hex::FromHexError;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("0x") || s.starts_with("0X") {
            s = &s[2..];
        }
        hex::decode(s).map(Self)
    }
}

impl ops::Deref for Hex {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for Hex {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Hex> for Vec<u8> {
    fn from(value: Hex) -> Self {
        value.0
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Hex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct("Hex", &self.to_string())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Hex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex = <&str as serde::Deserialize>::deserialize(deserializer)?;
        <Hex as std::str::FromStr>::from_str(hex).map_err(|err| <D::Error as serde::de::Error>::custom(err))
    }
}

impl fmt::Display for Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("0x")?;
        hex::encode(&self.0).fmt(f)
    }
}

impl From<Hex> for Value {
    fn from(value: Hex) -> Self {
        Value::Bytes(value.0)
    }
}

impl TryInto<Hex> for Value {
    type Error = ParseError;

    fn try_into(self) -> Result<Hex, Self::Error> {
        match self {
            Value::Bytes(bytes) => Ok(Hex(bytes)),
            _ => Err(Self::Error::wrong_value(ValueType::Bytes, self)),
        }
    }
}

impl_try_into_option!(Hex);

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::Hex;

    #[test]
    fn to_string_and_from_str() {
        let hex = Hex::new("hello hex".as_bytes().to_vec());
        assert_eq!(hex.to_string().as_str(), "0x68656c6c6f20686578");
        assert_eq!(Hex::from_str(&hex.to_string()).unwrap(), hex);
        assert_eq!(Hex::try_from_hex(&hex.to_string()).unwrap(), hex);
    }
}
