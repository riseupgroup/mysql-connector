use crate::{connection::types::Column, error::ParseError, types::Value};

pub trait FromQueryResultMapping<ModelData: super::ModelData>: Default {
    fn set_mapping_inner(&mut self, column: &Column, name: &str, index: usize);

    fn set_mapping(&mut self, column: &Column, name: &str, index: usize) {
        self.set_mapping_inner(
            column,
            name.strip_prefix(ModelData::TABLE_WITH_POINT)
                .unwrap_or(name),
            index,
        )
    }

    fn from_columns(columns: &[Column]) -> Self {
        let mut this = Self::default();
        for (i, column) in columns.iter().enumerate() {
            this.set_mapping(column, column.name(), i);
        }
        this
    }
}

pub trait FromQueryResult: super::ModelData + Sized {
    type Mapping: FromQueryResultMapping<Self>;

    fn from_mapping_and_row(
        mapping: &Self::Mapping,
        row: &mut Vec<Value>,
    ) -> std::result::Result<Self, ParseError>;
}
