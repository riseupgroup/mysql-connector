mod active_reference;
mod active_value;
mod named_value;
mod update_model;

use {
    super::Model,
    crate::{error::Error, types::Value, Connection, Stream},
};

pub use {
    active_reference::ActiveReference, active_value::ActiveValue, named_value::NamedValue,
    update_model::UpdateModel,
};

#[allow(async_fn_in_trait)]
pub trait ActiveModel<ModelData: super::ModelData>: Default {
    async fn into_values<S: Stream>(
        self,
        conn: &mut Connection<S>,
    ) -> Result<Vec<NamedValue>, Error>;

    fn primary(&self) -> Option<Value>;

    async fn insert<S: Stream>(self, conn: &mut Connection<S>) -> Result<u64, Error>
    where
        Self: Sized,
    {
        let values = self.into_values(conn).await?;
        let stmt = NamedValue::into_insert(&values, ModelData::TABLE)?;
        let mut stmt = conn.prepare_statement(&stmt).await?;
        stmt.execute(&values).await.map(|x| x.last_insert_id())
    }
}

pub trait HasActiveModel: super::ModelData {
    type ActiveModel: ActiveModel<Self>;

    /// Create [`ActiveModel`] containing the model's data.
    ///
    /// If the model has a primary key that is auto increment, it has to be set to [`ActiveValue::Unset`]
    fn into_active_model(self) -> Self::ActiveModel;
}
