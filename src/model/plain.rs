use {
    super::{FromQueryResult, FromQueryResultMapping},
    crate::{connection::types::Column, error::ParseError, types::Value},
};

impl super::ModelData for Vec<Value> {
    const TABLE: &'static str = "";
    const TABLE_WITH_POINT: &'static str = "";
}

impl FromQueryResult for Vec<Value> {
    type Mapping = EmptyMapping;

    fn from_mapping_and_row(
        mapping: &Self::Mapping,
        row: &mut Vec<Value>,
    ) -> std::result::Result<Self, ParseError> {
        if row.len() != mapping.len() {
            return Err(ParseError::RowLengthMismatch);
        }
        Ok(std::mem::take(row))
    }
}

#[derive(Default)]
pub struct EmptyMapping(usize);

impl EmptyMapping {
    pub fn len(&self) -> usize {
        self.0
    }
}

impl FromQueryResultMapping<Vec<Value>> for EmptyMapping {
    fn set_mapping_inner(&mut self, _column: &Column, _name: &str, _index: usize) {
        self.0 += 1;
    }

    fn set_mapping(&mut self, _column: &Column, _name: &str, _index: usize) {
        self.0 += 1;
    }

    fn from_columns(columns: &[Column]) -> Self {
        Self(columns.len())
    }
}
