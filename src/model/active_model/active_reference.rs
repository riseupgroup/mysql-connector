use {
    super::{ActiveModel, NamedValue},
    crate::{model::Model, Connection, Error},
};

#[derive(Debug)]
pub enum ActiveReference<T: Model> {
    Set(T::Primary),
    Insert(T::ActiveModel),
    Unset,
}

impl<T: Model> ActiveReference<T> {
    pub async fn insert_named_value(
        self,
        vec: &mut Vec<NamedValue>,
        name: &'static str,
        conn: &mut Connection,
    ) -> Result<(), Error> {
        match self {
            Self::Set(id) => vec.push(NamedValue(name, id.into())),
            Self::Insert(model) => {
                let primary = model.primary();
                let last_insert_id = model.insert(conn).await?;
                vec.push(NamedValue(name, primary.unwrap_or(last_insert_id.into())));
            }
            Self::Unset => (),
        }
        Ok(())
    }
}

impl<T: Model> Default for ActiveReference<T> {
    fn default() -> Self {
        Self::Unset
    }
}
