use {
    super::ColumnType,
    crate::{bitflags::ColumnFlags, error::ParseError, packets::ColumnDef},
};

#[derive(Debug, Clone)]
pub struct Column {
    org_name: String,
    name: String,
    org_table: String,
    table: String,
    r#type: ColumnType,
    flags: ColumnFlags,
}

impl<'a> TryFrom<ColumnDef<'a>> for Column {
    type Error = ParseError;

    fn try_from(value: ColumnDef<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            org_name: String::from_utf8(value.org_name.as_bytes().to_vec())?,
            name: String::from_utf8(value.name.as_bytes().to_vec())?,
            org_table: String::from_utf8(value.org_table.as_bytes().to_vec())?,
            table: String::from_utf8(value.table.as_bytes().to_vec())?,
            r#type: value.r#type,
            flags: value.flags,
        })
    }
}

impl Column {
    pub fn org_name(&self) -> &str {
        &self.org_name
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn org_table(&self) -> &str {
        &self.org_table
    }

    pub fn table(&self) -> &str {
        &self.table
    }

    pub fn r#type(&self) -> ColumnType {
        self.r#type
    }

    pub fn flags(&self) -> ColumnFlags {
        self.flags
    }
}
