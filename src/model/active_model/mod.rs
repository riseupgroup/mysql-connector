mod active_value;
mod named_value;
mod update_model;

use {
    super::Model,
    crate::{error::Error, Connection, Socket},
};

pub use {active_value::ActiveValue, named_value::NamedValue, update_model::UpdateModel};

#[allow(async_fn_in_trait)]
pub trait ActiveModel<ModelData: super::ModelData>: Default {
    fn into_values(self) -> Result<Vec<NamedValue>, Error>;

    async fn insert<S: Socket>(self, conn: &mut Connection<S>) -> Result<u64, Error>
    where
        Self: Sized,
    {
        let values = self.into_values()?;
        let stmt = NamedValue::into_insert(&values, ModelData::TABLE)?;
        let mut stmt = conn.prepare_statement(&stmt).await?;
        stmt.execute(&values).await.map(|x| x.last_insert_id())
    }
}

pub trait HasActiveModel: super::ModelData {
    type ActiveModel: ActiveModel<Self>;
}
