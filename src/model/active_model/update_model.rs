use {
    super::{ActiveModel, Model, NamedValue},
    crate::{error::Error, types::Value, Connection, Socket},
    std::ops::{Deref, DerefMut},
};

#[derive(Debug)]
pub struct UpdateModel<T: Model> {
    id: Value,
    model: T::ActiveModel,
}

impl<T: Model> UpdateModel<T> {
    pub fn new(id: Value) -> Self {
        Self {
            id,
            model: T::ActiveModel::default(),
        }
    }

    pub async fn update<S: Socket>(self, conn: &mut Connection<S>) -> Result<(), Error> {
        let mut values = self.model.into_values()?;
        if !values.is_empty() {
            let stmt = NamedValue::into_update(&values, T::TABLE, T::PRIMARY)?;
            values.push(NamedValue("", self.id));
            let mut stmt = conn.prepare_statement(&stmt).await?;
            stmt.execute(&values).await.map(|_| ())
        } else {
            Ok(())
        }
    }
}

impl<T: Model> Deref for UpdateModel<T> {
    type Target = T::ActiveModel;

    fn deref(&self) -> &Self::Target {
        &self.model
    }
}

impl<T: Model> DerefMut for UpdateModel<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.model
    }
}
