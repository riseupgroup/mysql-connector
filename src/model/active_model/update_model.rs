use {
    super::{ActiveModel, Model, NamedValue},
    crate::{error::Error, Connection},
    std::ops::{Deref, DerefMut},
};

#[derive(Debug)]
pub struct UpdateModel<T: Model> {
    id: T::Primary,
    model: T::ActiveModel,
}

impl<T: Model> UpdateModel<T> {
    pub fn new(id: T::Primary) -> Self {
        Self {
            id,
            model: T::ActiveModel::default(),
        }
    }

    pub async fn update(self, conn: &mut Connection) -> Result<(), Error> {
        let mut values = self.model.into_values(conn).await?;
        if !values.is_empty() {
            let stmt = NamedValue::into_update(&values, T::TABLE, T::PRIMARY)?;
            values.push(NamedValue("", self.id.into()));
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
