mod active_model;
mod from_query_result;
mod plain;

#[cfg(test)]
mod test;

pub use {
    active_model::{ActiveModel, ActiveValue, HasActiveModel, NamedValue, UpdateModel},
    from_query_result::{FromQueryResult, FromQueryResultMapping},
};

pub trait ModelData: std::fmt::Debug + Sized {
    const TABLE: &'static str;
    const TABLE_WITH_POINT: &'static str;
}

pub trait Model: ModelData + HasActiveModel {
    const PRIMARY: &'static str;

    type Primary;

    fn primary(&self) -> crate::types::Value;

    fn active_model() -> <Self as HasActiveModel>::ActiveModel {
        <Self as HasActiveModel>::ActiveModel::default()
    }

    fn update_model(&self) -> UpdateModel<Self> {
        UpdateModel::new(self.primary())
    }
}
