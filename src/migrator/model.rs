use {
    super::Version,
    crate::{
        connection::types::Column,
        error::ParseError,
        model::{FromQueryResult, FromQueryResultMapping, ModelData},
        types::Value,
    },
};

#[derive(Debug)]
pub(super) struct MigrationModel {
    pub(super) version: Version,
    pub(super) name: String,
}

impl ModelData for MigrationModel {
    const TABLE: &'static str = "migrations";
    const TABLE_WITH_POINT: &'static str = "migrations.";
}

impl FromQueryResult for MigrationModel {
    type Mapping = MigrationMapping;

    fn from_mapping_and_row(
        mapping: &Self::Mapping,
        row: &mut Vec<Value>,
    ) -> std::result::Result<Self, crate::error::ParseError> {
        Ok(Self {
            version: Version(
                row[mapping
                    .version_0
                    .ok_or(ParseError::MissingField("version_0"))?]
                .take()
                .try_into()?,
                row[mapping
                    .version_1
                    .ok_or(ParseError::MissingField("version_1"))?]
                .take()
                .try_into()?,
                row[mapping
                    .version_2
                    .ok_or(ParseError::MissingField("version_2"))?]
                .take()
                .try_into()?,
            ),
            name: row[mapping.name.ok_or(ParseError::MissingField("name"))?]
                .take()
                .try_into()?,
        })
    }
}

#[derive(Default)]
pub(super) struct MigrationMapping {
    version_0: Option<usize>,
    version_1: Option<usize>,
    version_2: Option<usize>,
    name: Option<usize>,
}

impl FromQueryResultMapping<MigrationModel> for MigrationMapping {
    fn set_mapping_inner(&mut self, column: &Column, _name: &str, index: usize) {
        *match column.org_name() {
            "version_0" => &mut self.version_0,
            "version_1" => &mut self.version_1,
            "version_2" => &mut self.version_2,
            "name" => &mut self.name,
            _ => return,
        } = Some(index);
    }
}
