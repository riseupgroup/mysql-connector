mod active_model;
mod from_query_result;
mod into_query;
mod plain;

#[cfg(test)]
mod test;

use crate::types::Value;

pub use {
    active_model::{
        ActiveModel, ActiveReference, ActiveValue, HasActiveModel, NamedValue, UpdateModel,
    },
    from_query_result::{FromQueryResult, FromQueryResultMapping},
    into_query::{IntoQuery, QueryColumn, QueryColumnReference},
};

pub trait ModelData: std::fmt::Debug + Sized {
    const TABLE: &'static str;
    const TABLE_WITH_POINT: &'static str;
}

pub trait Model: ModelData + HasActiveModel {
    const PRIMARY: &'static str;
    const AUTO_INCREMENT: bool;

    type Primary: Into<Value>;

    fn primary(&self) -> Self::Primary;

    fn active_model() -> <Self as HasActiveModel>::ActiveModel {
        <Self as HasActiveModel>::ActiveModel::default()
    }

    fn update_model(&self) -> UpdateModel<Self> {
        UpdateModel::new(self.primary())
    }
}
