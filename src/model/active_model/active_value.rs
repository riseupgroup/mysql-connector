use {
    super::NamedValue,
    crate::{error::SerializeError, types::Value},
};

#[derive(Debug)]
pub enum ActiveValue<T> {
    Set(T),
    Unset,
}

impl<T> ActiveValue<T>
where
    T: TryInto<Value>,
    <T as TryInto<Value>>::Error: Into<SerializeError>,
{
    pub fn insert_named_value(
        self,
        vec: &mut Vec<NamedValue>,
        name: &'static str,
    ) -> Result<(), SerializeError> {
        match self {
            Self::Set(value) => vec.push(NamedValue(name, value.try_into().map_err(Into::into)?)),
            Self::Unset => (),
        }
        Ok(())
    }
}

impl<T> Default for ActiveValue<T> {
    fn default() -> Self {
        Self::Unset
    }
}
